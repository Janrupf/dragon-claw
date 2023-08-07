import 'package:go_router/go_router.dart';

import 'home.dart';

class DragonClawRouter {
  /// Global instance of the router.
  static final GoRouter instance = GoRouter(routes: [
    GoRoute(path: "/", builder: (context, state) => const HomeScreen()),
  ]);
}
