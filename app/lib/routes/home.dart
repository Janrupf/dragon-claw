import 'package:dragon_claw/client/agent_store.dart';
import 'package:dragon_claw/components/add_device_dialog.dart';
import 'package:dragon_claw/components/device_list.dart';
import 'package:dragon_claw/updater/updater.dart';
import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart' as url_launcher;

final _logger = Logger("home");

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  late final AppUpdater _updater;
  late final KnownAgentStore _store;
  AvailableUpdate? _update;

  @override
  void initState() {
    super.initState();
    _updater = Provider.of<AppUpdater>(context, listen: false);
    _store = KnownAgentStore();

    _checkForUpdates();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text(
            "Dragon Claw",
            style: Theme.of(context).textTheme.headlineLarge,
          ),
        ),
        body: DeviceList(store: _store),
        bottomSheet: _buildUpdateSheet(context),
        floatingActionButton: FloatingActionButton(
          onPressed: () => _onAddPressed(context),
          child: const Icon(Icons.add),
        ),
      );

  Widget? _buildUpdateSheet(BuildContext context) {
    if (_update == null) {
      return null;
    }

    return ListTile(
      leading: const Icon(Icons.update),
      title: const Text("Update available"),
      subtitle: Text("Version ${_update!.version}"),
      trailing: TextButton(
        child: const Text("Update"),
        onPressed: () {
          url_launcher.launchUrl(
            Uri.parse(_update!.url),
            mode: url_launcher.LaunchMode.externalApplication,
          );
        },
      ),
    );
  }

  void _checkForUpdates() {
    _updater.checkForUpdates().then((update) {
      setState(() {
        _update = update;
      });
    }, onError: _updateCheckFailed);
  }

  void _updateCheckFailed(
    Object? error,
    StackTrace trace,
  ) {
    _logger.warning("Update check failed", error, trace);

    // Notify the user via snackbar
    if (context.mounted) {
      final colorScheme = Theme.of(context).colorScheme;

      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        duration: const Duration(days: 9999),
        backgroundColor: colorScheme.error,
        content: Text(
          "Update error: ${error.toString()}",
          style: TextStyle(color: colorScheme.onError),
        ),
        action: SnackBarAction(
          label: "Retry",
          onPressed: _checkForUpdates,
        ),
      ));
    }
  }

  void _onAddPressed(BuildContext context) {
    showDialog(
        context: context,
        builder: (context) {
          return AddDeviceDialog(store: _store);
        });
  }
}
