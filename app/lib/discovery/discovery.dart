import 'dart:collection';
import 'dart:io';

import 'package:dragon_claw/discovery/agent.dart';
import 'package:dragon_claw/discovery/ssdp.dart';
import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';
import 'package:nsd/nsd.dart' as nsd;

const _mDNSServiceName = "_dragon-claw._tcp";
const _ssdpServiceName = "urn:dragon-claw:service:DragonClawAgent:1";

final _log = Logger("discovery");

class DragonClawAgentDiscovery with ChangeNotifier {
  final Set<DiscoveredAgent> _discoveredAgents;
  late final SSDPDiscovery _ssdp;

  /// Retrieves the current list of discovered agents.
  UnmodifiableListView<DiscoveredAgent> get discoveredAgents =>
      UnmodifiableListView(_discoveredAgents);

  nsd.Discovery? _mDNS;

  DragonClawAgentDiscovery() : _discoveredAgents = HashSet() {
    _ssdp = SSDPDiscovery(
      serviceName: _ssdpServiceName,
      callback: _onSSDPMessage,
    );
  }

  /// Starts the discovery process.
  void start() async {
    if (_mDNS != null) {
      _log.warning("Discovery already running, ignoring start() call.");
      return;
    }
    _log.fine("Starting network discovery of agents...");

    _mDNS = await nsd.startDiscovery(
      _mDNSServiceName,
      ipLookupType: nsd.IpLookupType.any,
    );

    await _ssdp.start();

    _mDNS!.addServiceListener((service, status) {
      _log.fine("Service $service changed status to $status");
      final agent = _mapServiceToAgent(service);

      if (agent == null) {
        // This should not happen, but we log it just in case
        _log.warning("Could not derive agent from service $service");
        return;
      }

      final bool alive;
      switch (status) {
        case nsd.ServiceStatus.found:
          alive = true;
          break;

        case nsd.ServiceStatus.lost:
          alive = false;
          break;
      }

      _onAgentChanged(alive, agent);
    });
  }

  void _onSSDPMessage(SSDPStatus status, DiscoveredAgent agent) {
    final bool alive;
    switch (status) {
      case SSDPStatus.alive:
        alive = true;
        break;

      case SSDPStatus.byebye:
        alive = false;
        break;
    }

    _onAgentChanged(alive, agent);
  }

  void _onAgentChanged(bool alive, DiscoveredAgent agent) {
    if (alive) {
      _discoveredAgents.add(agent);
    } else {
      _discoveredAgents.remove(agent);
    }

    _log.finest("New list of discovered agents: $_discoveredAgents");
    notifyListeners();
  }

  void stop() async {
    _ssdp.stop();

    if (_mDNS == null) {
      _log.warning("Discovery not running, ignoring stop() call.");
      return;
    }

    _log.fine("Stopping network discovery of agents...");
    await nsd.stopDiscovery(_mDNS!);
    _mDNS = null;
  }

  DiscoveredAgent? _mapServiceToAgent(nsd.Service service) {
    final serviceAddress = service.addresses?.fold<InternetAddress?>(null,
        (previousValue, element) {
      // Prefer IPv4 addresses
      if (previousValue != null &&
          previousValue.type == InternetAddressType.IPv4) {
        return previousValue;
      }

      return element;
    });

    if (serviceAddress == null) {
      _log.warning("Service $service has no address, ignoring.");
      return null;
    }

    if (service.port == null) {
      _log.warning("Service $service has no port, ignoring.");
      return null;
    }

    return DiscoveredAgent(
      service.name ?? "<unknown>",
      serviceAddress,
      service.port!,
    );
  }
}
