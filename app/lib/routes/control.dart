import 'package:dragon_claw/client/agent_client.dart';
import 'package:dragon_claw/components/action.dart';
import 'package:dragon_claw/components/control/power_action_sheet.dart';
import 'package:dragon_claw/discovery/agent.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc.dart';
import 'package:logging/logging.dart';

final _log = Logger("screen.control");

class _AvailableOptions {
  final Set<PowerAction> powerActions;

  _AvailableOptions({required this.powerActions});

  /// Loads the available options from the agent.
  static Future<_AvailableOptions> load(DragonClawAgentClient client) async {
    final powerActions = await client.getSupportedPowerActions();
    return _AvailableOptions(powerActions: powerActions);
  }
}

class ControlScreen extends StatefulWidget {
  final DiscoveredAgent agent;

  /// The client constructed from the agent
  final DragonClawAgentClient _client;

  ControlScreen({required this.agent, super.key})
      : _client = DragonClawAgentClient(agent.address, agent.port);

  @override
  State<ControlScreen> createState() => _ControlScreenState();
}

class _ControlScreenState extends State<ControlScreen> {
  _AvailableOptions? _availableOptions;

  @override
  void initState() {
    super.initState();

    _AvailableOptions.load(widget._client).then((value) {
      if (mounted) {
        setState(() {
          _availableOptions = value;
        });
      }
    });
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text("Control ${widget.agent.name}"),
        ),
        body: _availableOptions == null
            ? _buildLoading(context)
            : _buildBody(context, _availableOptions!),
      );

  /// Build the widget content while still loading the available options.
  Widget _buildLoading(BuildContext context) => const Center(
        child: CircularProgressIndicator(),
      );

  /// Build the body of the widget with the available options.
  Widget _buildBody(BuildContext context, _AvailableOptions options) =>
      _ControlScreenContent(
        client: widget._client,
        options: options,
      );
}

class _ControlScreenContent extends StatelessWidget {
  final DragonClawAgentClient client;
  final _AvailableOptions options;

  const _ControlScreenContent({
    required this.client,
    required this.options,
  });

  @override
  Widget build(BuildContext context) => ListView(
        children: [
          if (options.powerActions.isNotEmpty) _buildPowerAction(context),
        ],
      );

  Widget _buildPowerAction(BuildContext context) {
    final defaultAction = options.powerActions.reduce((value, element) {
      // Prefer the action with the lowest index
      if (value.index < element.index) {
        return value;
      } else {
        return element;
      }
    });

    // Extract and sort the other actions
    final otherActions = options.powerActions
        .where((element) => element != defaultAction)
        .toList(growable: false);

    otherActions.sort((a, b) => a.index.compareTo(b.index));

    return IntrinsicHeight(
      child: ActionWithOptions(
        child: ListTile(
          leading: Icon(defaultAction.icon),
          title: Text(defaultAction.name),
          subtitle: Text(defaultAction.description),
        ),
        onPressed: () => _performPowerAction(defaultAction, context),
        onOptionsPressed: () {
          showModalBottomSheet<PowerAction>(
            showDragHandle: true,
            useRootNavigator: true,
            isDismissible: true,
            context: context,
            builder: (context) => PowerActionSheet(actions: otherActions),
          ).then((action) {
            if (action != null) {
              _performPowerAction(action, context);
            }
          });
        },
      ),
    );
  }

  void _performPowerAction(PowerAction action, BuildContext context) {
    _log.info("Performing power action $action");
    _handleRpcFut(client.performPowerAction(action), context);
  }

  Future<T> _handleRpcFut<T>(Future<T> rpcFut, BuildContext context) {
    return rpcFut.then((value) {
      if (context.mounted) {
        _notifySuccess(context, "Power action performed");
      }
      return value;
    }).catchError((error, trace) {
      if (context.mounted) {
        _onRpcError(context, error, trace);
      }

      return Future<T>.error(error, trace);
    });
  }

  void _notifySuccess(BuildContext context, String message) {
    // Show an error snackbar based on the color scheme
    final snackBar = SnackBar(content: Text(message));

    ScaffoldMessenger.of(context).showSnackBar(snackBar);
  }

  void _onRpcError(BuildContext context, Object error, StackTrace trace) {
    _log.severe("RPC error", error, trace);

    // Attempt to generate a sensible error message with a fallback
    final String errorMessage;
    if (error is GrpcError) {
      errorMessage = error.message ?? error.toString();
    } else {
      errorMessage = error.toString();
    }

    final colorScheme = Theme.of(context).colorScheme;

    // Show an error snackbar based on the color scheme
    final snackBar = SnackBar(
      content: Text(errorMessage, style: TextStyle(color: colorScheme.onError)),
      backgroundColor: colorScheme.error,
    );

    ScaffoldMessenger.of(context).showSnackBar(snackBar);
  }
}
