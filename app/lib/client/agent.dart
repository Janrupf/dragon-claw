import 'dart:io';

/// Discovered dragon claw agent.
class KnownAgent {
  final String name;
  final InternetAddress address;
  final int port;

  /// If [true], the agent was automatically discovered using network discovery.
  /// Otherwise it was manually added.
  final bool discovered;

  KnownAgent.discovered(this.name, this.address, this.port) : discovered = true;

  KnownAgent.manual(this.name, this.address, this.port) : discovered = false;

  KnownAgent toManual() => KnownAgent.manual(name, address, port);

  @override
  String toString() {
    return 'DiscoveredAgent{name: $name, address: $address, port: $port}';
  }

  @override
  int get hashCode => Object.hash(name, address, port);

  @override
  bool operator ==(Object other) =>
      other is KnownAgent &&
      other.name == name &&
      other.address == address &&
      other.port == port;
}
