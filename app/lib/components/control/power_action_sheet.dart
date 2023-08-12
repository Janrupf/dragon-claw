import 'package:flutter/material.dart';

class PowerActionSheet extends StatelessWidget {
  const PowerActionSheet({super.key});

  @override
  Widget build(BuildContext context) => ListView(
        children: [
          ListTile(
            leading: const Icon(Icons.power_settings_new),
            title: const Text("Power off"),
            subtitle: const Text("Shut down the system"),
            onTap: () =>
                Navigator.pop(context, PowerActionSheetResult.powerOff),
          ),
          ListTile(
            leading: const Icon(Icons.restart_alt),
            title: const Text("Restart"),
            subtitle: const Text("Restart the system"),
            onTap: () => Navigator.pop(context, PowerActionSheetResult.restart),
          ),
          ListTile(
            leading: const Icon(Icons.logout),
            title: const Text("Log out"),
            subtitle: const Text("Log out of the current user"),
            onTap: () => Navigator.pop(context, PowerActionSheetResult.logout),
          ),
        ],
      );
}

enum PowerActionSheetResult {
  powerOff,
  restart,
  logout,
}
