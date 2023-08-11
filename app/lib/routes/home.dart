import 'package:dragon_claw/components/discovery_list.dart';
import 'package:flutter/material.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text(
            "Dragon Claw",
            style: Theme.of(context).textTheme.headlineLarge,
          ),
        ),
        body: const DiscoveryList(),
      );
}
