import 'dart:io';

import 'package:dragon_claw/client/agent.dart';
import 'package:dragon_claw/client/agent_store.dart';
import 'package:flutter/material.dart';

/// Dialog to add a new device.
class AddDeviceDialog extends StatefulWidget {
  final KnownAgentStore store;

  const AddDeviceDialog({
    super.key,
    required this.store,
  });

  @override
  State<StatefulWidget> createState() => _AddDeviceDialogState();
}

class _AddDeviceDialogState extends State<AddDeviceDialog> {
  final _formKey = GlobalKey<FormState>();
  List<KnownAgent>? _manualAgents;

  final _nameController = TextEditingController();
  final _addressController = TextEditingController();
  final _portController = TextEditingController();

  @override
  void initState() {
    super.initState();

    // Load the manually added agents so we can avoid duplicates
    widget.store.load().then((value) => setState(() {
          _manualAgents = value;
        }));
  }

  @override
  Widget build(BuildContext context) =>
      Dialog.fullscreen(child: _buildContent(context));

  Widget _buildContent(BuildContext context) {
    final Widget child;
    if (_manualAgents == null) {
      child = const Center(
        child: CircularProgressIndicator(),
      );
    } else {
      child = _buildForm(context);
    }

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: () => Navigator.of(context).pop(),
        ),
        actions: [
          TextButton(
              onPressed: _manualAgents == null ? null : _onSavePressed,
              child: const Text("Save")),
        ],
        title: const Text("Add device"),
      ),
      body: child,
    );
  }

  Widget _buildForm(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Form(
        key: _formKey,
        child: Column(
          children: [
            TextFormField(
              controller: _nameController,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: "Device name",
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return "Please enter a name";
                }

                return null;
              },
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _addressController,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: "IP address",
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return "Please enter an IP address";
                }

                try {
                  InternetAddress(value);
                  return null;
                } catch (e) {
                  return "Please enter a valid IP address";
                }
              },
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _portController,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                labelText: "Port",
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return "Please enter a port";
                }

                final port = int.tryParse(value);
                if (port == null || port < 0 || port > 65535) {
                  return "Please enter a valid port";
                }

                return null;
              },
            ),
          ],
        ),
      ),
    );
  }

  void _onSavePressed() {
    if (_formKey.currentState!.validate()) {
      final name = _nameController.text;
      final address = _addressController.text;
      final port = int.parse(_portController.text);

      final agent = KnownAgent.manual(name, InternetAddress(address), port);

      final newAgentList = List.of(_manualAgents!);
      newAgentList.add(agent);

      widget.store.save(newAgentList);

      Navigator.of(context).pop();
    }
  }
}
