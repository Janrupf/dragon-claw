import 'package:dragon_claw/routing.dart';
import 'package:flutter/material.dart';
import 'package:logging/logging.dart';

void main() {
  Logger.root.level = Level.ALL;
  Logger.root.onRecord.listen((record) {
    print("${record.level.name}: ${record.time}: ${record.message}");

    if (record.error != null) {
      print(record.error);
    }

    if (record.stackTrace != null) {
      final trace = record.stackTrace.toString();

      if (trace.isNotEmpty) {
        print("Stacktrace: $trace");
      }
    }
  });

  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      title: 'Dragon Claw',
      theme: ThemeData(
        colorScheme: const ColorScheme.dark(),
        useMaterial3: true,
      ),
      routerConfig: DragonClawRouter.instance,
    );
  }
}