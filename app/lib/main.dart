import 'package:dragon_claw/routing.dart';
import 'package:dragon_claw/updater/updater.dart';
import 'package:dynamic_color/dynamic_color.dart';
import 'package:flutter/material.dart';
import 'package:logging/logging.dart';
import 'package:provider/provider.dart';

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

  runApp(Provider(
    create: (context) => AppUpdater(),
    child: const MyApp(),
  ));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) => DynamicColorBuilder(
        builder: (lightScheme, darkScheme) {
          ColorScheme scheme;
          if (MediaQuery.of(context).platformBrightness == Brightness.light) {
            scheme = lightScheme ?? const ColorScheme.light();
          } else {
            scheme = darkScheme ?? const ColorScheme.dark();
          }

          return MaterialApp.router(
            title: 'Dragon Claw',
            theme: ThemeData(
              colorScheme: scheme,
              useMaterial3: true,
              snackBarTheme: const SnackBarThemeData(
                // Material 3, no idea why flutter doesn't already sets this
                behavior: SnackBarBehavior.floating,
              ),
            ),
            routerConfig: DragonClawRouter.instance,
          );
        },
      );
}
