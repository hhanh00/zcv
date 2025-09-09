import 'package:flutter/material.dart';
import 'package:zcv/src/rust/api/rpc.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => HomePageState();
}

class HomePageState extends State<HomePage> {
  int r = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('ZCV Home Page'),
        actions: [
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: () async {
              await getBlockRange(start: 2100000, end: 3050000);
            },
          ),
        ],
      ),
      body: Center(
        child: Text('Welcome to ZCV! Counter: $r')),
    );
  }
}
