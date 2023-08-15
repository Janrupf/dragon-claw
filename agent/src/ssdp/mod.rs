use socket2::{Domain, Protocol, Socket};
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

const SSDP_ANY_IPV4: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const SSDP_ANY_IPV4_SOCKET: SocketAddr = SocketAddr::V4(SocketAddrV4::new(SSDP_ANY_IPV4, 1900));
const SSDP_ANY_IPV6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);
const SSDP_ANY_IPV6_SOCKET: SocketAddr =
    SocketAddr::V6(SocketAddrV6::new(SSDP_ANY_IPV6, 1900, 0, 0));
const SSDP_MULTICAST_IPV4: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_MULTICAST_IPV4_SOCKET: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(SSDP_MULTICAST_IPV4, 1900));
const SSDP_MULTICAST_IPV6: Ipv6Addr = Ipv6Addr::new(0xff05, 0, 0, 0, 0, 0, 0, 0x000c);
const SSDP_MULTICAST_IPV6_SOCKET: SocketAddr =
    SocketAddr::V6(SocketAddrV6::new(SSDP_MULTICAST_IPV6, 1900, 0, 0));

const SSDP_SERVICE_TYPE: &str = "urn:dragon-claw:service:DragonClawAgent:1";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum IpAddrWithScopeId {
    V4(Ipv4Addr),
    V6 { addr: Ipv6Addr, scope_id: u32 },
}

impl IpAddrWithScopeId {
    /// Returns true if the address is an IPv4 address.
    pub fn is_ipv4(&self) -> bool {
        match self {
            IpAddrWithScopeId::V4(_) => true,
            IpAddrWithScopeId::V6 { .. } => false,
        }
    }

    /// Derives an `IpAddrWithScopeId` from a `SocketAddr`.
    pub fn derive_from(addr: &SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v) => IpAddrWithScopeId::V4(*v.ip()),
            SocketAddr::V6(v) => IpAddrWithScopeId::V6 {
                addr: *v.ip(),
                scope_id: v.scope_id(),
            },
        }
    }

    /// Converts the address to a `SocketAddr` with the given port.
    pub fn to_socket_addr(self, port: u16) -> SocketAddr {
        match self {
            IpAddrWithScopeId::V4(v) => SocketAddr::V4(SocketAddrV4::new(v, port)),
            IpAddrWithScopeId::V6 { addr, scope_id } => {
                SocketAddr::V6(SocketAddrV6::new(addr, port, 0, scope_id))
            }
        }
    }

    /// Returns true if the address is a loopback address.
    pub fn is_loopback(&self) -> bool {
        match self {
            IpAddrWithScopeId::V4(v) => v.is_loopback(),
            IpAddrWithScopeId::V6 { addr, .. } => addr.is_loopback(),
        }
    }
}

impl Display for IpAddrWithScopeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V4(addr) => Display::fmt(addr, f),
            Self::V6 { addr, scope_id } => {
                write!(f, "[{}%{}]", addr, scope_id)
            }
        }
    }
}

#[derive(Debug)]
struct SendTask {
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
    notify: Arc<Notify>,
    addr: SocketAddr,
}

impl SendTask {
    fn new(
        socket: UdpSocket,
        shutdown: Arc<AtomicBool>,
        notify: Arc<Notify>,
        addr: SocketAddr,
    ) -> Self {
        Self {
            socket,
            shutdown,
            notify,
            addr,
        }
    }
}

#[derive(Debug)]
struct SendTaskNotifiers {
    shutdown: Arc<AtomicBool>,
    notifiers: Vec<Arc<Notify>>,
}

#[derive(Debug)]
pub struct SSDPMulticast {
    notifiers: SendTaskNotifiers,
    send_task: JoinHandle<()>,
    receive_task: JoinHandle<()>,
}

impl SSDPMulticast {
    /// Set's up the SSDP multicast and begins SSDP multicast announcements
    /// for the given service address.
    pub async fn setup<F>(
        service_addr: SocketAddr,
        get_local_addresses: F,
    ) -> Result<Self, std::io::Error>
    where
        F: FnOnce() -> Result<Vec<IpAddrWithScopeId>, std::io::Error>,
    {
        let is_unspecified = service_addr.ip().is_unspecified();

        // Determine the local addresses we need to listen for SSDP multicast requests on
        let (local_ipv4, local_ipv6) = match is_unspecified {
            true => get_local_addresses()?,
            false => vec![IpAddrWithScopeId::derive_from(&service_addr)],
        }
        .iter()
        .filter(|v| !v.is_loopback())
        .partition::<Vec<IpAddrWithScopeId>, _>(|v| v.is_ipv4());

        // Attempt to bind a multicast receiver for each address family
        let mut receive_sockets = Vec::new();
        let mut send_tasks = Vec::new();

        let shutdown = Arc::new(AtomicBool::new(false));

        Self::bind_multicast_sockets(
            &local_ipv4,
            &mut receive_sockets,
            &mut send_tasks,
            shutdown.clone(),
            service_addr.port(),
        );
        Self::bind_multicast_sockets(
            &local_ipv6,
            &mut receive_sockets,
            &mut send_tasks,
            shutdown.clone(),
            service_addr.port(),
        );

        if receive_sockets.is_empty() {
            // Could not bind to any local interface for receiving SSDP multicast requests
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "No local addresses to bind to",
            ));
        }

        let notifiers = send_tasks
            .iter()
            .map(|v| v.notify.clone())
            .collect::<Vec<_>>();

        let send_task = tokio::spawn(Self::send_task(send_tasks));
        let receive_task = tokio::spawn(Self::receive_task(receive_sockets, notifiers.clone()));

        let notifiers = SendTaskNotifiers {
            shutdown,
            notifiers,
        };

        Ok(Self {
            notifiers,
            send_task,
            receive_task,
        })
    }

    fn bind_multicast_sockets(
        local_addresses: &[IpAddrWithScopeId],
        receive_sockets: &mut Vec<UdpSocket>,
        send_tasks: &mut Vec<SendTask>,
        shutdown: Arc<AtomicBool>,
        service_port: u16,
    ) {
        let receiver =
            match Self::bind_multicast_receiver(local_addresses).and_then(Self::socket2_to_tokio) {
                Some(socket) => socket,
                None => return,
            };

        let len_before = send_tasks.len();
        send_tasks.extend(
            local_addresses
                .iter()
                .map(|&a| Self::bind_multicast_sender(a).map(|v| (a, v)))
                .filter_map(|v| v.and_then(|(a, v)| Self::socket2_to_tokio(v).map(|v| (a, v))))
                .map(|(a, s)| {
                    SendTask::new(
                        s,
                        shutdown.clone(),
                        Arc::new(Notify::new()),
                        a.to_socket_addr(service_port),
                    )
                }),
        );

        if len_before == send_tasks.len() {
            // No senders were created for this receiver
            return;
        }

        receive_sockets.push(receiver);
    }

    fn bind_multicast_receiver(local_addresses: &[IpAddrWithScopeId]) -> Option<Socket> {
        let domain = match local_addresses.first() {
            // No socket to bind in this address family
            None => return None,
            Some(IpAddrWithScopeId::V4(_)) => Domain::IPV4,
            Some(IpAddrWithScopeId::V6 { .. }) => Domain::IPV6,
        };

        let do_bind = move || -> Result<Socket, std::io::Error> {
            // Create a single UDP socket for receiving SSDP multicast requests from all local addresses
            let socket = Socket::new(domain, socket2::Type::DGRAM, Some(Protocol::UDP))?;

            // Configure the socket
            socket.set_reuse_address(true)?;
            #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
            socket.set_reuse_port(true)?;

            // Bind the socket to the multicast address
            match domain {
                Domain::IPV4 => {
                    socket.bind(&SSDP_MULTICAST_IPV4_SOCKET.into())?;
                    #[cfg(unix)]
                    socket.set_multicast_loop_v4(false)?;
                }
                Domain::IPV6 => {
                    socket.bind(&SSDP_MULTICAST_IPV6_SOCKET.into())?;
                    #[cfg(unix)]
                    socket.set_multicast_loop_v6(false)?;
                }
                _ => unreachable!(),
            }

            // Join the multicast group on all local addresses
            for local_address in local_addresses {
                let res = match (domain, local_address) {
                    (Domain::IPV4, IpAddrWithScopeId::V4(addr)) => {
                        socket.join_multicast_v4(&SSDP_MULTICAST_IPV4, addr)
                    }
                    (Domain::IPV6, IpAddrWithScopeId::V6 { scope_id, .. }) => {
                        socket.join_multicast_v6(&SSDP_MULTICAST_IPV6, *scope_id)
                    }
                    _ => panic!("Attempted to mix IPv4 and IPv6 addresses"),
                };

                if let Err(err) = res {
                    tracing::warn!(
                        "Failed to join multicast group on {}: {}",
                        local_address,
                        err
                    );
                }
            }

            Ok(socket)
        };

        match do_bind() {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("Failed to bind multicast receiver: {}", e);
                None
            }
        }
    }

    fn bind_multicast_sender(address: IpAddrWithScopeId) -> Option<Socket> {
        let do_bind = move || -> Result<Socket, std::io::Error> {
            let socket = match address {
                IpAddrWithScopeId::V4(_) => Socket::new(Domain::IPV4, socket2::Type::DGRAM, None)?,
                IpAddrWithScopeId::V6 { .. } => {
                    Socket::new(Domain::IPV6, socket2::Type::DGRAM, None)?
                }
            };

            // Configure the socket
            socket.set_reuse_address(true)?;
            #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
            socket.set_reuse_port(true)?;

            // Bind the socket to the multicast address
            match address {
                IpAddrWithScopeId::V4(addr) => {
                    socket.set_multicast_if_v4(&addr)?;
                    socket.set_multicast_loop_v4(false)?;
                    socket.bind(&SSDP_ANY_IPV4_SOCKET.into())?
                }
                IpAddrWithScopeId::V6 { scope_id, .. } => {
                    socket.set_multicast_if_v6(scope_id)?;
                    socket.set_multicast_loop_v6(false)?;
                    socket.bind(&SSDP_ANY_IPV6_SOCKET.into())?;
                }
            }

            Ok(socket)
        };

        match do_bind() {
            Ok(v) => Some(v),
            Err(err) => {
                tracing::warn!("Failed to bind multicast sender: {}", err);
                None
            }
        }
    }

    fn socket2_to_tokio(socket: Socket) -> Option<UdpSocket> {
        let do_convert = move || -> Result<UdpSocket, std::io::Error> {
            socket.set_nonblocking(true)?;
            UdpSocket::from_std(std::net::UdpSocket::from(socket))
        };

        match do_convert() {
            Ok(v) => Some(v),
            Err(err) => {
                tracing::warn!("Failed to convert socket to tokio: {}", err);
                None
            }
        }
    }

    fn build_ssdp_message(service_addr: SocketAddr, nts: &str) -> Vec<u8> {
        // Host to announce in the SSDP message
        let host = match service_addr.ip() {
            IpAddr::V4(_) => "239.255.255.250:1900",
            IpAddr::V6(_) => "[ff05::c]:1900",
        };

        // Build the SSDP request
        let http_request = http::Request::builder()
            .method("NOTIFY")
            .uri("*")
            .version(http::Version::HTTP_11)
            .header("HOST", host)
            .header("NT", SSDP_SERVICE_TYPE)
            .header(
                "USER-AGENT",
                concat!("DragonClaw/", env!("CARGO_PKG_VERSION")),
            )
            .header("NTS", nts)
            .header("CACHE-CONTROL", "max-age=30")
            .header("LOCATION", format!("tcp://{}", service_addr))
            .header("MAN", "\"ssdp:discover\"")
            .body(())
            .unwrap();

        Self::http_request_to_data(http_request)
    }

    /// Helper function to always send an entire buffer over a UDP socket
    async fn send_all_to(
        socket: &UdpSocket,
        data: &[u8],
        multicast_address: &SocketAddr,
    ) -> Result<(), std::io::Error> {
        let mut send_index = 0;

        while send_index < data.len() {
            let bytes_sent = socket
                .send_to(&data[send_index..], &multicast_address)
                .await?;

            send_index += bytes_sent;
        }

        Ok(())
    }

    async fn send_task(sockets: Vec<SendTask>) {
        async fn send_loop(
            SendTask {
                addr,
                socket,
                shutdown,
                notify,
            }: SendTask,
        ) {
            let port = match socket.local_addr() {
                Ok(v) => v.port(),
                Err(err) => {
                    tracing::warn!("Failed to get local port: {}", err);
                    return;
                }
            };

            let multicast_address = if addr.is_ipv4() {
                SSDP_MULTICAST_IPV4_SOCKET
            } else {
                SSDP_MULTICAST_IPV6_SOCKET
            };
            let alive_data = SSDPMulticast::build_ssdp_message(addr, "ssdp:alive");

            loop {
                // Make sure we always write out the entire request
                if let Err(err) =
                    SSDPMulticast::send_all_to(&socket, &alive_data, &multicast_address).await
                {
                    tracing::warn!("Failed to send SSDP alive request for {}: {}", addr, err);
                }
                tracing::trace!(
                    "Sent SSDP request for {}, sleeping for 30 seconds (or until notify)",
                    addr
                );

                // Send SSDP request every 30 seconds or until we are notified
                tokio::select!(
                    _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {},
                    _ = notify.notified() => {},
                );

                if shutdown.load(Ordering::Acquire) {
                    // Shutdown requested
                    break;
                }
            }

            let byebye_data = SSDPMulticast::build_ssdp_message(addr, "ssdp:byebye");
            if let Err(err) =
                SSDPMulticast::send_all_to(&socket, &byebye_data, &multicast_address).await
            {
                tracing::warn!("Failed to send SSDP byebye request: {}", err);
            } else {
                tracing::debug!("Sent SSDP byebye request for {}", addr);
            }
        }

        // Send SSDP requests on all sockets
        futures::future::join_all(sockets.into_iter().map(send_loop)).await;
    }

    async fn receive_task(sockets: Vec<UdpSocket>, notifiers: Vec<Arc<Notify>>) {
        async fn receive_loop(socket: UdpSocket, notifiers: &[Arc<Notify>]) {
            let mut receive_buffer = Vec::with_capacity(1024);

            loop {
                match socket.recv_buf(&mut receive_buffer).await {
                    Ok(v) => v,
                    Err(err) => {
                        tracing::warn!("Failed to receive SSDP response: {}", err);
                        continue;
                    }
                };

                let requests = SSDPMulticast::data_to_http_request(&mut receive_buffer);

                if receive_buffer.len() > 4096 {
                    // Either someone is attempting to send us a very large SSDP requests or
                    // we only received parts of many requests. Either way, clear the buffer.
                    tracing::warn!("Receive buffer has grown too large, clearing");
                    receive_buffer.clear();
                }

                let mut do_notify = false;

                for request in requests {
                    let method = request.method();

                    // Check if there is a search request for dragon_claw_agent
                    if method.as_str() == "M-SEARCH" {
                        let st = request.headers().get("ST").and_then(|v| v.to_str().ok());
                        if st.is_some_and(|st| st == SSDP_SERVICE_TYPE) {
                            do_notify = true;
                            tracing::trace!("Received SSDP search for dragon_claw_agent");
                        }
                    };
                }

                if do_notify {
                    // Notify the senders to send a new SSDP request
                    for notify in notifiers {
                        notify.notify_waiters();
                    }
                }
            }
        }

        // Receive SSDP requests on all sockets
        futures::future::join_all(sockets.into_iter().map(|s| receive_loop(s, &notifiers))).await;
    }

    /// Stops the ssdp multicast announcements.
    pub async fn stop(self) {
        // We can just abort the receive tasks, they don't need to do any cleanup
        self.receive_task.abort();

        // For the send tasks, we need to notify them to stop and then wait for them to finish
        self.notifiers.shutdown.store(true, Ordering::Release);
        for notifier in self.notifiers.notifiers {
            notifier.notify_waiters();
        }

        let _ = tokio::join!(self.receive_task, self.send_task);
    }

    /// Converts an HTTP request into a byte vector.
    fn http_request_to_data(request: http::Request<()>) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(request.method().as_str().as_bytes());
        data.push(b' ');
        data.extend_from_slice(request.uri().path().as_bytes());
        data.push(b' ');
        data.extend_from_slice(match request.version() {
            http::Version::HTTP_09 => b"HTTP/0.9",
            http::Version::HTTP_10 => b"HTTP/1.0",
            http::Version::HTTP_11 => b"HTTP/1.1",
            http::Version::HTTP_2 => b"HTTP/2.0",
            http::Version::HTTP_3 => b"HTTP/3.0",
            _ => unreachable!("Unsupported HTTP version"),
        });
        data.extend_from_slice(b"\r\n");
        for (name, value) in request.headers() {
            data.extend_from_slice(name.as_str().to_uppercase().as_bytes());
            data.extend_from_slice(b": ");
            data.extend_from_slice(value.as_bytes());
            data.extend_from_slice(b"\r\n");
        }
        data.extend_from_slice(b"\r\n");
        data
    }

    /// This is a very bad implementation of an HTTP request parser - but its fault tolerant
    /// and should work for our use case since it only needs to parse somewhat simple SSDP
    /// requests.
    fn data_to_http_request(data: &mut Vec<u8>) -> Vec<http::Request<()>> {
        let mut found_requests = Vec::new();

        loop {
            let Some(end_of_response) = Self::find_subsequence(data, b"\r\n\r\n") else { break };
            // Keep the \r\n so we can detect end of lines
            let mut response_data = &data[..(end_of_response + 2)];

            let mut request = http::Request::builder();
            let mut begin_found = false;

            // Go over all lines in the response
            loop {
                let Some(end_of_line) = Self::find_subsequence(response_data, b"\r\n") else { break };
                let line = &response_data[..end_of_line];

                if line.is_empty() {
                    // Ignore empty lines
                    continue;
                }

                // Advance the response data
                response_data = &response_data[end_of_line + 2..];

                if !begin_found {
                    // We need the http version, uri and http version
                    let mut parts = line.splitn(3, |c| *c == b' ');
                    let Some(http_method) = parts.next() else { continue; };
                    let Some(uri) = parts.next() else { continue; };
                    let Some(http_version) = parts.next() else { continue; };

                    // Parse the http begin
                    let Ok(method) = http::Method::from_bytes(http_method) else { continue; };
                    #[allow(clippy::unnecessary_to_owned)] // Is not unnecessary here
                    let Ok(uri)  = http::uri::Uri::from_maybe_shared(uri.to_owned()) else { continue; };
                    let version = match http_version {
                        b"HTTP/0.9" => http::Version::HTTP_09,
                        b"HTTP/1.0" => http::Version::HTTP_10,
                        b"HTTP/1.1" => http::Version::HTTP_11,
                        b"HTTP/2.0" => http::Version::HTTP_2,
                        b"HTTP/3.0" => http::Version::HTTP_3,
                        _ => continue, // Unsupported HTTP version (or we got a HTTP response)
                    };

                    // Set the fields on the request
                    request = request.method(method).uri(uri).version(version);

                    begin_found = true;
                } else {
                    // Find the colon in the line
                    let Some(colon_index) = Self::find_subsequence(line, b":") else { continue; };
                    let (name, value) = line.split_at(colon_index);

                    // Parse the header name
                    let Ok(name) = http::header::HeaderName::from_bytes(Self::trim_slice(name)) else { continue; };
                    let Ok(value) = http::header::HeaderValue::from_bytes(Self::trim_slice(&value[1..])) else { continue; };

                    // Set the header on the request
                    request = request.header(name, value);
                }
            }

            if begin_found {
                // Build the request
                let request = request.body(()).unwrap();

                // Add the request to the list of found requests
                found_requests.push(request);
            }

            // Remove the response from the data
            data.drain(..end_of_response + 4);
        }

        found_requests
    }

    /// Finds a subsequence in a byte slice.
    ///
    /// The way this is defined may seem a bit weird first, but this allows the compiler to
    /// apply a good chunk of optimizations to this function.
    fn find_subsequence<const SIZE: usize>(data: &[u8], sequence: &[u8; SIZE]) -> Option<usize> {
        if data.len() < SIZE {
            return None;
        }

        let mut search_index = 0;
        let searchable_len = data.len() - SIZE;

        while search_index <= searchable_len {
            // Construct a slice of the next SIZE bytes
            let search_slice = &data[search_index..search_index + SIZE];
            if search_slice == sequence {
                return Some(search_index);
            }

            search_index += 1;
        }

        None
    }

    /// Trims leading and trailing whitespace from a slice.
    fn trim_slice(input: &[u8]) -> &[u8] {
        let mut start_index = 0;
        let mut end_index = input.len();

        // Find the first non-whitespace character
        while start_index < end_index {
            if !input[start_index].is_ascii_whitespace() {
                break;
            }

            start_index += 1;
        }

        // Find the last non-whitespace character
        while end_index > start_index {
            if !input[end_index - 1].is_ascii_whitespace() {
                break;
            }

            end_index -= 1;
        }

        &input[start_index..end_index]
    }
}
