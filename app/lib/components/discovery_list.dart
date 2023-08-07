import 'package:dragon_claw/discovery/discovery.dart';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class DiscoveryList extends StatefulWidget {
  const DiscoveryList({super.key});

  @override
  State<StatefulWidget> createState() => _DiscoveryListState();
}

class _DiscoveryListState extends State<DiscoveryList> {
  /// The [DragonClawAgentDiscovery] to use for discovery.
  late final DragonClawAgentDiscovery _discovery;

  @override
  void initState() {
    super.initState();

    _discovery = DragonClawAgentDiscovery();
    _discovery.addListener(_onChange);
    _discovery.start();
  }

  @override
  void dispose() {
    _discovery.stop();
    _discovery.removeListener(_onChange);

    super.dispose();
  }

  void _onChange() {
    setState(() {
      /* rebuild */
    });
  }

  @override
  Widget build(BuildContext context) {
    final count = _discovery.discoveredAgents.length;

    if (count > 0) {
      return ListView.builder(
        itemCount: _discovery.discoveredAgents.length,
        itemBuilder: _buildEntry,
      );
    } else {
      return const Center(
        child: Text("No agents found."),
      );
    }
  }

  Widget _buildEntry(BuildContext context, int index) {
    final agent = _discovery.discoveredAgents[index];

    return ListTile(
      title: Text(agent.name),
      subtitle: Text(agent.address.address),
      onTap: () {
        context.push("/control", extra: agent);
      },
    );
  }
}
