import 'package:dragon_claw/client/agent_client.dart';
import 'package:flutter/material.dart';

class PowerActionSheet extends StatelessWidget {
  final List<PowerAction> actions;

  const PowerActionSheet({super.key, required this.actions});

  @override
  Widget build(BuildContext context) =>
      ListView(children: _buildListViewItems(context));

  List<Widget> _buildListViewItems(BuildContext context) => actions
      .map((action) => ListTile(
            leading: Icon(action.icon),
            title: Text(action.name),
            subtitle: Text(action.description),
            onTap: () => _onTapAction(action, context),
          ))
      .toList(growable: false);

  void _onTapAction(PowerAction action, BuildContext context) {
    Navigator.of(context).pop(action);
  }
}
