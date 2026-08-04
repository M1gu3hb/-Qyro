import 'package:flutter/material.dart';

import 'boot/boot_screen.dart';
import 'home/home_screen.dart';

class QyroApp extends StatefulWidget {
  const QyroApp({super.key});

  @override
  State<QyroApp> createState() => _QyroAppState();
}

class _QyroAppState extends State<QyroApp> {
  var _showBoot = true;

  @override
  Widget build(BuildContext context) {
    const colors = ColorScheme.dark(
      primary: Color(0xFF168BFF),
      secondary: Color(0xFF51C8FF),
      surface: Color(0xFF07111D),
    );

    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'Qyro',
      theme: ThemeData(
        colorScheme: colors,
        scaffoldBackgroundColor: const Color(0xFF03070D),
        useMaterial3: true,
      ),
      home: _showBoot
          ? BootScreen(
              onFinished: () {
                if (mounted) {
                  setState(() => _showBoot = false);
                }
              },
            )
          : const HomeScreen(),
    );
  }
}
