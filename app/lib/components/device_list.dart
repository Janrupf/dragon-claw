import 'package:dragon_claw/client/agent.dart';
import 'package:dragon_claw/client/agent_store.dart';
import 'package:dragon_claw/discovery/discovery.dart';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

class DeviceList extends StatefulWidget {
  final KnownAgentStore store;

  const DeviceList({super.key, required this.store});

  @override
  State<StatefulWidget> createState() => _DeviceListState();
}

class _DeviceListState extends State<DeviceList> {
  /// The [DragonClawAgentDiscovery] to use for discovery.
  late final DragonClawAgentDiscovery _discovery;
  List<KnownAgent>? _manualAgents;

  @override
  void initState() {
    super.initState();

    widget.store.addListener(_onStoreChange);
    _onStoreChange();

    _discovery = DragonClawAgentDiscovery();
    _discovery.addListener(_onChange);
    _discovery.start();
  }

  @override
  void dispose() {
    _discovery.stop();
    _discovery.removeListener(_onChange);

    widget.store.removeListener(_onStoreChange);

    super.dispose();
  }

  void _onStoreChange() {
    widget.store.load().then((value) {
      setState(() {
        _manualAgents = value;
      });
    });
  }

  void _onChange() {
    setState(() {
      /* rebuild */
    });
  }

  @override
  Widget build(BuildContext context) {
    final allAgents = List<KnownAgent>.of(_manualAgents ?? []);
    for (final agent in _discovery.discoveredAgents) {
      if (!allAgents.contains(agent)) {
        allAgents.add(agent);
      }
    }

    final count = allAgents.length;

    if (count > 0) {
      return ListView.builder(
        itemCount: count,
        itemBuilder: (context, index) => _buildEntry(context, index, allAgents),
      );
    } else {
      return const Center(
        child: Text("No agents found."),
      );
    }
  }

  Widget _buildEntry(BuildContext context, int index, List<KnownAgent> agents) {
    final theme = Theme.of(context).textTheme;
    final agent = agents[index];

    return InkWell(
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: ListTile(
          leading: const Icon(Icons.computer),
          title: Text(
            agent.name,
            style: theme.headlineSmall,
          ),
          subtitle: Text("${agent.address.address}:${agent.port}"),
          trailing: _buildTrailing(agents[index]),
        ),
      ),
      onTap: () {
        context.push("/control", extra: agent);
      },
    );
  }

  /// Builds the bookmark/un-bookmark button for the agent.
  Widget? _buildTrailing(KnownAgent agent) {
    if (agent.discovered) {
      return IconButton(
        onPressed: () {
          final newAgentList = List.of(_manualAgents!);
          newAgentList.add(agent.toManual());
          widget.store.save(newAgentList);
        },
        icon: const Icon(Icons.bookmark_border),
      );
    } else {
      return IconButton(
        onPressed: () {
          final newAgentList = List.of(_manualAgents!);
          newAgentList.remove(agent);
          widget.store.save(newAgentList);
        },
        icon: const Icon(Icons.bookmark),
      );
    }
  }
}
