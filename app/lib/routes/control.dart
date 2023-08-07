import 'package:dragon_claw/discovery/agent.dart';
import 'package:flutter/material.dart';

class ControlScreen extends StatelessWidget {
  final DiscoveredAgent agent;

  const ControlScreen({required this.agent, super.key});

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: const Text("Dragon Claw"),
        ),
        body: const Center(
          child: Text("Control"),
        ),
      );
}
