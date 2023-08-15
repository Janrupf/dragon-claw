import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:dragon_claw/discovery/agent.dart';
import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';

final _logger = Logger("ssdp:discovery");

/// Callback for when a new agent is discovered.
typedef SSDPDiscoveryCallback = void Function(
    SSDPStatus status, DiscoveredAgent agent);

/// Status associated with an SSDP message.
enum SSDPStatus {
  /// The service is alive
  alive,

  /// The service is shutting down
  byebye
}

/// Standard headers used in SSDP messages.
class SSDPStandardHeaders {
  const SSDPStandardHeaders._();

  // The service notification type
  static final SSDPHeaderName nt = SSDPHeaderName.fromString("NT");

  /// The service notification subtype
  static final SSDPHeaderName nts = SSDPHeaderName.fromString("NTS");

  /// The service location
  static final SSDPHeaderName location = SSDPHeaderName.fromString("LOCATION");

  /// The search target
  static final SSDPHeaderName st = SSDPHeaderName.fromString("ST");

  /// The unique service name
  static final SSDPHeaderName usn = SSDPHeaderName.fromString("USN");
}

/// Header names used in SSDP messages.
class SSDPHeaderName {
  /// The raw header name.
  final Uint8List raw;

  const SSDPHeaderName(this.raw);

  SSDPHeaderName.fromString(String name) : raw = utf8.encode(name);

  /// The header name as a string.
  String? get name {
    try {
      return utf8.decode(raw);
    } on FormatException {
      return null;
    }
  }

  @override
  int get hashCode => Object.hashAll(raw);

  @override
  bool operator ==(Object other) {
    if (other is SSDPHeaderName) {
      return listEquals(raw, other.raw);
    }

    return false;
  }
}

/// Header values used in SSDP messages.
class SSDPHeaderValue {
  /// The raw header value.
  final Uint8List raw;

  const SSDPHeaderValue(this.raw);

  SSDPHeaderValue.fromString(String value) : raw = utf8.encode(value);

  /// The header value as a string.
  String? get value {
    try {
      return utf8.decode(raw);
    } on FormatException {
      return null;
    }
  }

  @override
  int get hashCode => Object.hashAll(raw);

  @override
  bool operator ==(Object other) {
    if (other is SSDPHeaderValue) {
      return listEquals(raw, other.raw);
    }

    return false;
  }
}

class SSDPDiscovery {
  static final InternetAddress ssdpMulticastIPv4Address =
      InternetAddress("239.255.255.250");
  static final InternetAddress ssdpMulticastIPv6Address =
      InternetAddress("FF05::C");
  static const ssdpMulticastPort = 1900;

  /// The name of the service to discover.
  final String serviceName;
  final SSDPDiscoveryCallback callback;

  final List<RawDatagramSocket> _sendSockets;
  final List<RawDatagramSocket> _receiveSockets;

  final List<_SSDPSender> _senders;
  final List<StreamSubscription<RawSocketEvent>> _socketSubscriptions;

  bool get running => _running;
  bool _running = false;

  /// Constructs a new SSDP discovery instance.
  SSDPDiscovery({required this.serviceName, required this.callback})
      : _sendSockets = [],
        _receiveSockets = [],
        _senders = [],
        _socketSubscriptions = [];

  /// Starts the discovery process.
  Future<void> start() async {
    if (_running) {
      _logger.warning("Discovery already running, ignoring start() call.");
      return;
    }
    _running = true;

    // Attempt to list all network interfaces
    final networkInterfaces = await NetworkInterface.list();

    final ipv4Interfaces = HashSet<NetworkInterface>();
    final ipv6Interfaces = HashSet<NetworkInterface>();

    // Iterate over all network interfaces and associate them with IPv4
    // and/or IPv6
    for (final networkInterface in networkInterfaces) {
      for (final address in networkInterface.addresses) {
        if (address.type == InternetAddressType.IPv4) {
          ipv4Interfaces.add(networkInterface);
        } else if (address.type == InternetAddressType.IPv6) {
          ipv6Interfaces.add(networkInterface);
        }
      }
    }

    // Bind senders and receivers to available network interfaces
    await Future.wait([
      _bindReceiver(true, ipv4Interfaces)
          .then((receiver) => _completeBinding(true, receiver, ipv4Interfaces)),
      _bindReceiver(false, ipv6Interfaces).then(
          (receiver) => _completeBinding(false, receiver, ipv6Interfaces)),
    ]);

    if (_receiveSockets.isEmpty) {
      _logger
          .warning("No receivers bound, will not be able to discover agents");
      _running = false;
      return;
    }

    // Register receivers on all receive sockets
    for (final socket in _receiveSockets) {
      final receiver = _SSDPReceiver(socket, _onSSDPMessage);
      final subscription = socket.listen(receiver.onEvent);
      _socketSubscriptions.add(subscription);
    }

    // Start all senders
    for (final socket in _sendSockets) {
      final sender = _SSDPSender(socket, serviceName);
      _senders.add(sender);

      final subscription = socket.listen(
        sender.onEvent,
        onError: sender.onError,
        cancelOnError: false,
      );
      _socketSubscriptions.add(subscription);
    }
  }

  /// Stops the discovery process.
  void stop() {
    if (!_running) {
      _logger.warning("Discovery not running, ignoring stop() call.");
      return;
    }

    // Stop all senders
    for (final sender in _senders) {
      sender.stop();
    }

    // Cancel all subscriptions
    for (final subscription in _socketSubscriptions) {
      subscription.cancel();
    }

    // Close all sockets
    for (final socket in _sendSockets) {
      socket.close();
    }

    for (final socket in _receiveSockets) {
      socket.close();
    }

    _senders.clear();
    _socketSubscriptions.clear();
    _sendSockets.clear();
    _receiveSockets.clear();

    _running = false;
  }

  /// Attempts to bind UDP senders when an UDP receiver has been bound.
  Future<void> _completeBinding(
    bool isIPv4,
    RawDatagramSocket? receiver,
    Set<NetworkInterface> listenInterfaces,
  ) async {
    if (receiver == null) {
      // Nothing to do
      return;
    }

    _receiveSockets.add(receiver);

    // Bind senders to all listen interfaces
    final senders = await _bindSenders(isIPv4, listenInterfaces);
    if (senders.isEmpty) {
      final familyName = isIPv4 ? "IPv4" : "IPv6";
      _logger.warning(
        "Bound an $familyName receiver, but no senders, "
        "will only passively discover on $familyName",
      );
    } else {
      _sendSockets.addAll(senders);
    }
  }

  /// Binds a UDP receiver to the multicast address and joins the multicast
  /// group on all listen interfaces.
  Future<RawDatagramSocket?> _bindReceiver(
    bool isIPv4,
    Set<NetworkInterface> listenInterfaces,
  ) async {
    if (listenInterfaces.isEmpty) {
      // No addresses to listen on
      return null;
    }

    // Bind to the multicast address
    final receiverSocket = await RawDatagramSocket.bind(
      isIPv4 ? ssdpMulticastIPv4Address : ssdpMulticastIPv6Address,
      ssdpMulticastPort,
      reuseAddress: true,
      reusePort: true,
    );

    // Disable loopback and write events
    receiverSocket.multicastLoopback = false;
    receiverSocket.writeEventsEnabled = false;

    // Now join the multicast on all listen interfaces
    for (final interface in listenInterfaces) {
      try {
        receiverSocket.joinMulticast(
          isIPv4 ? ssdpMulticastIPv4Address : ssdpMulticastIPv6Address,
          interface,
        );
      } catch (e, trace) {
        _logger.warning(
          "Failed to join multicast group on interface ${interface.name}: $e",
          e,
          trace,
        );
      }
    }

    return receiverSocket;
  }

  /// Binds multiple UDP sockets to the given interfaces for sending SSDP
  /// multicast messages.
  Future<List<RawDatagramSocket>> _bindSenders(
    bool isIPv4,
    Set<NetworkInterface> listenInterfaces,
  ) async {
    final encounteredIpv6Indices = HashSet<int>();
    final sockets = <RawDatagramSocket>[];

    for (final interface in listenInterfaces) {
      for (final address in interface.addresses) {
        if (address.type !=
            (isIPv4 ? InternetAddressType.IPv4 : InternetAddressType.IPv6)) {
          // Skip addresses that don't match the requested type
          continue;
        }

        if (!isIPv4 && encounteredIpv6Indices.contains(interface.index)) {
          // Skip IPv6 addresses if we already encountered this interface
          // index
          continue;
        }
        encounteredIpv6Indices.add(interface.index);

        final RawDatagramSocket senderSocket;
        try {
          // Create a new socket for each interface
          senderSocket = await RawDatagramSocket.bind(
            isIPv4 ? InternetAddress.anyIPv4 : InternetAddress.anyIPv6,
            ssdpMulticastPort,
            reuseAddress: true,
            reusePort: true,
          );
        } catch (e, trace) {
          _logger.warning(
            "Failed to bind sender socket to interface ${interface.name}: $e",
            e,
            trace,
          );
          continue;
        }

        // Disable loopback, read events and set the multicast interface
        senderSocket.multicastLoopback = false;
        senderSocket.readEventsEnabled = false;

        try {
          if (isIPv4) {
            senderSocket.setRawOption(RawSocketOption(
              RawSocketOption.levelIPv4,
              RawSocketOption.IPv4MulticastInterface,
              address.rawAddress,
            ));
          } else {
            senderSocket.setRawOption(RawSocketOption.fromInt(
              RawSocketOption.levelIPv6,
              RawSocketOption.IPv6MulticastInterface,
              interface.index,
            ));
          }
        } catch (e, trace) {
          senderSocket.close();

          _logger.warning(
            "Failed to set multicast interface on sender socket for interface ${interface.name}: $e",
            e,
            trace,
          );

          continue;
        }

        sockets.add(senderSocket);
      }
    }

    return sockets;
  }

  void _onSSDPMessage(
    String httpMethod,
    String uri,
    String httpVersion,
    Map<SSDPHeaderName, SSDPHeaderValue> headers,
  ) {
    if (httpMethod == "NOTIFY" &&
        uri == "*" &&
        httpVersion == "HTTP/1.1" &&
        headers[SSDPStandardHeaders.nt]?.value == serviceName) {
      // SSDP NOTIFY message for our service
      final location = headers[SSDPStandardHeaders.location]?.value;
      final subtype = headers[SSDPStandardHeaders.nts]?.value;

      if (location == null || subtype == null) {
        _logger.warning(
          "Received NOTIFY message for service $serviceName, but it is missing "
          "the location or subtype header",
        );
        return;
      }

      final locationUri = Uri.tryParse(location);
      if (locationUri == null) {
        _logger.warning(
          "Received NOTIFY message for service $serviceName, but the location "
          "header is not a valid URI: $location",
        );
        return;
      }

      final SSDPStatus status;
      switch (subtype) {
        case "ssdp:alive":
          status = SSDPStatus.alive;
          break;

        case "ssdp:byebye":
          status = SSDPStatus.byebye;
          break;

        default:
          _logger.warning(
            "Received NOTIFY message for service $serviceName, but it has an "
            "unknown subtype: $subtype",
          );
          return;
      }

      // Construct the agent
      final name =
          headers[SSDPStandardHeaders.usn]?.value ?? "Dragon Claw Computer";
      final agent = DiscoveredAgent(
          name, InternetAddress(locationUri.host), locationUri.port);

      // Notify the callback
      callback(status, agent);
    }
  }
}

typedef _SSDPMessageReceivedCallback = void Function(
  String httpMethod,
  String uri,
  String httpVersion,
  Map<SSDPHeaderName, SSDPHeaderValue> headers,
);

/// Helper for receiving SSDP messages.
class _SSDPReceiver {
  final Uint8List _messageEnd =
      Uint8List.fromList([13, 10, 13, 10] /* \r\n\r\n */);
  final Uint8List _lineEnd = Uint8List.fromList([13, 10] /* \r\n */);

  final _SSDPMessageReceivedCallback receivedCallback;
  final RawDatagramSocket socket;
  Uint8List? _buffer;

  _SSDPReceiver(this.socket, this.receivedCallback);

  void onEvent(RawSocketEvent event) async {
    if (event != RawSocketEvent.read) {
      // Ignore all events except read
      return;
    }

    try {
      await _doRead();
    } finally {
      // Enable read events again
      socket.readEventsEnabled = true;
    }
  }

  /// Performs the actual read from the socket.
  Future<void> _doRead() async {
    final datagram = socket.receive();
    if (datagram == null) {
      _logger.warning("Read ready, but no datagram available");
      return;
    }

    // Append or store the data
    final buffer = datagram.data;
    if (_buffer == null) {
      _buffer = buffer;
    } else {
      _buffer = Uint8List(_buffer!.length + buffer.length)
        ..setAll(0, _buffer!)
        ..setAll(_buffer!.length, buffer);
    }

    if (_buffer!.length > 4096) {
      // This should not happen during normal operation, but if it does, we
      // should clear the buffer to avoid potential DoS attacks.
      _logger.warning("Receive buffer has grown too large, clearing");
      _buffer = Uint8List(0);
    }

    int messageEnd;
    while ((messageEnd = _findUntilSubsequence(_buffer!, _messageEnd)) != -1) {
      // Leave \r\n at the end so we can split the message into lines
      Uint8List message = _buffer!.sublist(
        0,
        _findUntilSubsequence(_buffer!, _messageEnd) + (_messageEnd.length - 2),
      );

      // Remove the message from the buffer
      _buffer = _buffer!.sublist(messageEnd + _messageEnd.length);

      String? httpMethod;
      String? uri;
      String? httpVersion;
      final headers = HashMap<SSDPHeaderName, SSDPHeaderValue>();

      // Split the message into lines
      int lineEnd;
      while ((lineEnd = _findUntilSubsequence(message, _lineEnd)) != -1) {
        Uint8List line = message.sublist(0, lineEnd);
        message = message.sublist(lineEnd + _lineEnd.length);

        if (line.isEmpty) {
          continue;
        }

        if (httpMethod == null) {
          String lineString;
          try {
            lineString = utf8.decode(line, allowMalformed: false);
          } on FormatException {
            // Not a valid first line for a HTTP request, maybe we caught
            // the middle of a request.
            continue;
          }

          final parts = lineString.split(" ");
          if (parts.length != 3) {
            // Not valid
            continue;
          }

          httpMethod = parts[0];
          uri = parts[1];
          httpVersion = parts[2];
        } else {
          // Decode a header
          int indexOfColon = line.indexOf(':'.codeUnitAt(0));
          if (indexOfColon == -1) {
            // Not a valid header
            continue;
          }

          // Insert the header into the map
          final key = _trimUint8List(line.sublist(0, indexOfColon));
          final value = _trimUint8List(line.sublist(indexOfColon + 1));
          headers[SSDPHeaderName(key)] = SSDPHeaderValue(value);
        }
      }

      if (httpMethod != null && uri != null && httpVersion != null) {
        receivedCallback(httpMethod, uri, httpVersion, headers);
      }
    }
  }

  /// Attempt to sensibly trim a [Uint8List] by removing leading and trailing
  /// whitespace.
  Uint8List _trimUint8List(Uint8List list) {
    var start = 0;
    while (start < list.length && list[start] <= 32) {
      start++;
    }

    var end = list.length;
    while (end > start && list[end - 1] <= 32) {
      end--;
    }

    return list.sublist(start, end);
  }

  /// Finds the first occurrence of the given subsequence in the buffer and
  /// returns the data up to that point.
  int _findUntilSubsequence(Uint8List buffer, Uint8List subsequence) {
    if (buffer.length < subsequence.length) {
      // Not enough data to match
      return -1;
    }

    // Search for the subsequence
    for (var i = 0; i <= buffer.length - subsequence.length; i++) {
      var match = true;
      for (var j = 0; j < subsequence.length; j++) {
        if (buffer[i + j] != subsequence[j]) {
          match = false;
          break;
        }
      }

      if (match) {
        // Found a match
        return i;
      }
    }

    // No match found
    return -1;
  }
}

/// Helper for sending SSDP messages.
class _SSDPSender {
  final RawDatagramSocket socket;
  final String serviceName;
  final Uint8List _searchMessage;
  late final Timer _timer;

  bool _timerExpired = false;
  bool _writeReady = false;

  _SSDPSender(this.socket, this.serviceName)
      : _searchMessage = _buildMSearchMessage(serviceName) {
    _timer = Timer.periodic(const Duration(seconds: 5), _onTimerExpired);
  }

  void onEvent(RawSocketEvent event) {
    if (event != RawSocketEvent.write) {
      // Ignore all events except write
      return;
    }

    _writeReady = true;
    _attemptWrite();
  }

  void onError(Object error, StackTrace trace) {
    _logger.warning("Error while sending M-SEARCH: $error", error, trace);
    _logger.warning("Disabling this sender");
    stop();
  }

  void _onTimerExpired(Timer timer) {
    _timerExpired = true;
    _attemptWrite();
  }

  void _attemptWrite() {
    if (!_writeReady || !_timerExpired) {
      // Not ready to write
      return;
    }

    try {
      _doWrite();
    } finally {
      // Reset event state
      _writeReady = false;
      _timerExpired = false;
      socket.writeEventsEnabled = true;
    }
  }

  void _doWrite() {
    _logger.finest("Sending M-SEARCH...");

    final isIPv4 = socket.address.type == InternetAddressType.IPv4;
    socket.send(
      _searchMessage,
      isIPv4
          ? SSDPDiscovery.ssdpMulticastIPv4Address
          : SSDPDiscovery.ssdpMulticastIPv6Address,
      SSDPDiscovery.ssdpMulticastPort,
    );
  }

  /// Stops the sender
  void stop() {
    _timer.cancel();
  }

  static Uint8List _buildMSearchMessage(String serviceName) => utf8.encode(
        "M-SEARCH * HTTP/1.1\r\n"
        "MX: 5\r\n"
        "HOST: 239.255.255.250:1900\r\n"
        "MAN: \"ssdp:discover\"\r\n"
        "ST: $serviceName\r\n"
        "\r\n",
      );
}
