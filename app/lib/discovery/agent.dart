import 'dart:io';

/// Discovered dragon claw agent.
class DiscoveredAgent {
  final String name;
  final InternetAddress address;
  final int port;

  DiscoveredAgent(this.name, this.address, this.port);

  @override
  String toString() {
    return 'DiscoveredAgent{name: $name, address: $address, port: $port}';
  }

  @override
  int get hashCode => Object.hash(name, address, port);

  @override
  bool operator ==(Object other) =>
      other is DiscoveredAgent &&
      other.name == name &&
      other.address == address &&
      other.port == port;
}
