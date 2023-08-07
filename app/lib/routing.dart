import 'package:dragon_claw/discovery/agent.dart';
import 'package:dragon_claw/routes/control.dart';
import 'package:go_router/go_router.dart';

import 'routes/home.dart';

class DragonClawRouter {
  /// Global instance of the router.
  static final GoRouter instance = GoRouter(routes: [
    GoRoute(path: "/", builder: (context, state) => const HomeScreen()),
    GoRoute(
        path: "/control",
        builder: (context, state) {
          final agent = state.extra as DiscoveredAgent;

          return ControlScreen(agent: agent);
        }),
  ]);
}
