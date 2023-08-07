import 'dart:collection';
import 'dart:io';

import 'package:dragon_claw/discovery/agent.dart';
import 'package:flutter/foundation.dart';
import 'package:logging/logging.dart';
import 'package:nsd/nsd.dart' as nsd;

const _dragonClawAgentServiceName = "_dragon-claw._tcp";

final _log = Logger("discovery");

class DragonClawAgentDiscovery with ChangeNotifier {
  final List<DiscoveredAgent> _discoveredAgents = [];

  /// Retrieves the current list of discovered agents.
  UnmodifiableListView<DiscoveredAgent> get discoveredAgents =>
      UnmodifiableListView(_discoveredAgents);

  nsd.Discovery? _discovery;

  DragonClawAgentDiscovery();

  /// Starts the discovery process.
  void start() async {
    if (_discovery != null) {
      _log.warning("Discovery already running, ignoring start() call.");
      return;
    }
    _log.fine("Starting network discovery of agents...");

    _discovery = await nsd.startDiscovery(
      _dragonClawAgentServiceName,
      ipLookupType: nsd.IpLookupType.any,
    );

    _discovery!.addServiceListener((service, status) {
      _log.fine("Service $service changed status to $status");
      final agent = _mapServiceToAgent(service);

      if (agent == null) {
        // This should not happen, but we log it just in case
        _log.warning("Could not derive agent from service $service");
        return;
      }

      switch (status) {
        case nsd.ServiceStatus.found:
          _discoveredAgents.add(agent);
          break;

        case nsd.ServiceStatus.lost:
          _discoveredAgents.remove(agent);
          break;
      }

      _log.finest("New list of discovered agents: $_discoveredAgents");
      notifyListeners();
    });
  }

  void stop() async {
    if (_discovery == null) {
      _log.warning("Discovery not running, ignoring stop() call.");
      return;
    }

    _log.fine("Stopping network discovery of agents...");
    await nsd.stopDiscovery(_discovery!);
    _discovery = null;
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
