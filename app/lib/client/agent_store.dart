import 'dart:collection';
import 'dart:convert' as convert;
import 'dart:io';

import 'package:dragon_claw/client/agent.dart';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart' as path_provider;

class KnownAgentStore with ChangeNotifier {
  /// The future which resolves to the storage directory.
  final Future<Directory> _storageDirectory;
  late final Future<File> _knownAgentsFile;

  static Future<Directory> _getStorageDirectory() {
    return path_provider.getApplicationDocumentsDirectory();
  }

  List<KnownAgent>? _knownAgents;

  /// Asynchronously set's up the store.
  KnownAgentStore() : _storageDirectory = _getStorageDirectory() {
    _knownAgentsFile = _storageDirectory.then((value) {
      return File("${value.path}/known_agents.json");
    });
  }

  /// Loads the known agents from storage.
  Future<UnmodifiableListView<KnownAgent>> load() async {
    if (_knownAgents != null) {
      return UnmodifiableListView(_knownAgents!);
    }

    final file = await _knownAgentsFile;
    if (!await file.exists()) {
      return UnmodifiableListView([]);
    }

    final contents = await file.readAsString();
    final json = convert.jsonDecode(contents);

    if (json is! List) {
      throw const FormatException("Expected JSON to be a list");
    }

    final knownAgents = <KnownAgent>[];
    for (final item in json) {
      if (item is! Map) {
        throw const FormatException("Expected JSON to be a list of maps");
      }

      final name = item["name"];
      final address = item["address"];
      final port = item["port"];

      if (name is! String || address is! String || port is! int) {
        throw const FormatException(
            "Expected JSON to be a list of maps with name, address and port");
      }

      // Add the agent to the list
      knownAgents.add(KnownAgent.manual(name, InternetAddress(address), port));
    }

    _knownAgents = knownAgents;
    return UnmodifiableListView(knownAgents);
  }

  /// Saves the known agents to storage.
  Future<void> save(List<KnownAgent> knownAgents) async {
    _knownAgents = knownAgents;
    notifyListeners();

    final file = await _knownAgentsFile;
    final json = knownAgents
        .map((e) => {
              "name": e.name,
              "address": e.address.address,
              "port": e.port,
            })
        .toList(growable: false);

    await file.writeAsString(convert.jsonEncode(json));
  }
}
