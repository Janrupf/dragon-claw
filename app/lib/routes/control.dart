import 'package:dragon_claw/client/agent_client.dart';
import 'package:dragon_claw/discovery/agent.dart';
import 'package:flutter/material.dart';
import 'package:grpc/grpc.dart';
import 'package:logging/logging.dart';

final _log = Logger("screen.control");

class ControlScreen extends StatelessWidget {
  final DiscoveredAgent agent;

  /// The client constructed from the agent
  final DragonClawAgentClient _client;

  ControlScreen({required this.agent, super.key})
      : _client = DragonClawAgentClient(agent.address, agent.port);

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text("Control ${agent.name}"),
        ),
        body: Center(
          child: IconButton.filled(
            iconSize: 64,
            onPressed: () => _shutdownPressed(context),
            icon: const Icon(Icons.power_settings_new),
          ),
        ),
      );

  void _shutdownPressed(BuildContext context) {
    _client
        .shutdownSystem()
        .then((value) => _notifySuccess(context, "Request sent!"))
        .catchError((error, trace) => _onRpcError(context, error, trace));
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
