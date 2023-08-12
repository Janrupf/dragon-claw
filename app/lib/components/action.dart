import 'package:flutter/material.dart';

/// Widget which provides a primary action and a secondary action which can be
/// accessed by pressing the overflow button.
class ActionWithOptions extends StatelessWidget {
  final Widget child;
  final VoidCallback? onPressed;
  final VoidCallback? onOptionsPressed;

  const ActionWithOptions({
    required this.child,
    this.onPressed,
    this.onOptionsPressed,
    super.key,
  });

  @override
  Widget build(BuildContext context) => InkWell(
        onTap: onPressed,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(
              child: child,
            ),
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 16.0),
              child: VerticalDivider(),
            ),
            AspectRatio(
              aspectRatio: 1.0,
              child: IconButton(
                onPressed: onOptionsPressed,
                icon: const Icon(Icons.arrow_drop_down),
              ),
            )
          ],
        ),
      );
}
