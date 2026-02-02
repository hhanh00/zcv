// The original content is temporarily commented out to allow generating a self-contained demo - feel free to uncomment later.

// import 'package:flutter/material.dart';
//
// void main() {
//   runApp(const MyApp());
// }
//
// class MyApp extends StatelessWidget {
//   const MyApp({super.key});
//
//   // This widget is the root of your application.
//   @override
//   Widget build(BuildContext context) {
//     return MaterialApp(
//       title: 'Flutter Demo',
//       theme: ThemeData(
//         // This is the theme of your application.
//         //
//         // TRY THIS: Try running your application with "flutter run". You'll see
//         // the application has a purple toolbar. Then, without quitting the app,
//         // try changing the seedColor in the colorScheme below to Colors.green
//         // and then invoke "hot reload" (save your changes or press the "hot
//         // reload" button in a Flutter-supported IDE, or press "r" if you used
//         // the command line to start the app).
//         //
//         // Notice that the counter didn't reset back to zero; the application
//         // state is not lost during the reload. To reset the state, use hot
//         // restart instead.
//         //
//         // This works for code too, not just values: Most code changes can be
//         // tested with just a hot reload.
//         colorScheme: .fromSeed(seedColor: Colors.deepPurple),
//       ),
//       home: const MyHomePage(title: 'Flutter Demo Home Page'),
//     );
//   }
// }
//
// class MyHomePage extends StatefulWidget {
//   const MyHomePage({super.key, required this.title});
//
//   // This widget is the home page of your application. It is stateful, meaning
//   // that it has a State object (defined below) that contains fields that affect
//   // how it looks.
//
//   // This class is the configuration for the state. It holds the values (in this
//   // case the title) provided by the parent (in this case the App widget) and
//   // used by the build method of the State. Fields in a Widget subclass are
//   // always marked "final".
//
//   final String title;
//
//   @override
//   State<MyHomePage> createState() => _MyHomePageState();
// }
//
// class _MyHomePageState extends State<MyHomePage> {
//   int _counter = 0;
//
//   void _incrementCounter() {
//     setState(() {
//       // This call to setState tells the Flutter framework that something has
//       // changed in this State, which causes it to rerun the build method below
//       // so that the display can reflect the updated values. If we changed
//       // _counter without calling setState(), then the build method would not be
//       // called again, and so nothing would appear to happen.
//       _counter++;
//     });
//   }
//
//   @override
//   Widget build(BuildContext context) {
//     // This method is rerun every time setState is called, for instance as done
//     // by the _incrementCounter method above.
//     //
//     // The Flutter framework has been optimized to make rerunning build methods
//     // fast, so that you can just rebuild anything that needs updating rather
//     // than having to individually change instances of widgets.
//     return Scaffold(
//       appBar: AppBar(
//         // TRY THIS: Try changing the color here to a specific color (to
//         // Colors.amber, perhaps?) and trigger a hot reload to see the AppBar
//         // change color while the other colors stay the same.
//         backgroundColor: Theme.of(context).colorScheme.inversePrimary,
//         // Here we take the value from the MyHomePage object that was created by
//         // the App.build method, and use it to set our appbar title.
//         title: Text(widget.title),
//       ),
//       body: Center(
//         // Center is a layout widget. It takes a single child and positions it
//         // in the middle of the parent.
//         child: Column(
//           // Column is also a layout widget. It takes a list of children and
//           // arranges them vertically. By default, it sizes itself to fit its
//           // children horizontally, and tries to be as tall as its parent.
//           //
//           // Column has various properties to control how it sizes itself and
//           // how it positions its children. Here we use mainAxisAlignment to
//           // center the children vertically; the main axis here is the vertical
//           // axis because Columns are vertical (the cross axis would be
//           // horizontal).
//           //
//           // TRY THIS: Invoke "debug painting" (choose the "Toggle Debug Paint"
//           // action in the IDE, or press "p" in the console), to see the
//           // wireframe for each widget.
//           mainAxisAlignment: .center,
//           children: [
//             const Text('You have pushed the button this many times:'),
//             Text(
//               '$_counter',
//               style: Theme.of(context).textTheme.headlineMedium,
//             ),
//           ],
//         ),
//       ),
//       floatingActionButton: FloatingActionButton(
//         onPressed: _incrementCounter,
//         tooltip: 'Increment',
//         child: const Icon(Icons.add),
//       ),
//     );
//   }
// }
//

import 'package:flutter/material.dart';
import 'package:zcv/src/rust/api/simple.dart';
import 'package:zcv/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    final electionJSON = compileElectionDef(
      electionYaml: """{
  "start": 2978050,
  "end": 3218812,
  "need_sig": true,
  "name": "NU7 Sentiment Poll",
  "questions": [
    {
      "title": "What is your general sentiment toward including the following protocol features?",
      "choices": [
        {
          "title": "Zcash Shielded Assets (ZSAs)",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Network Sustainability Mechanism (NSM)",
          "subtitle": "including smoothing the issuance curve, which allows ZEC to be removed from circulation and later reissued as future block rewards to help sustain network security while preserving the 21 million ZEC supply cap",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Burning 60% of transaction fees via the NSM",
          "subtitle": "The goals are to demonstrate Zcash’s commitment to long-term sustainability, to burn ZEC so that it can be re-issued in the future without exceeding the 21M supply cap, and in the context of dynamic fees, to prevent miners from manipulating fees",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Memo Bundles",
          "subtitle": "which let transactions include memos larger than 512 bytes and share a memo across multiple recipients, and also permits inclusion of authenticated reply-to addresses",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Explicit Fees",
          "subtitle": "allowing transaction fees to be clearly specified and committed to in the transaction",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Disallowing v4 transactions",
          "subtitle": "reducing the complexity and attack surface of the Zcash protocol and would disable the ability to spend Sprout funds",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Project Tachyon",
          "subtitle": "a new shielded protocol or pool to address scalability challenges",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "STARK proof verification",
          "subtitle": "via Transparent Zcash Extensions (TZEs) to enable Layer-2 designs on Zcash",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Dynamic fee mechanism",
          "subtitle": "a comparable-based, dynamic fees",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Consensus accounts",
          "subtitle": "which generalize the functionality of the dev fund lockbox and reduce the operational expense of collecting ZCG funds and miner rewards",
          "answers": [
            "Support",
            "Oppose"
          ]
        },
        {
          "title": "Orchard quantum recoverability",
          "subtitle": "which aims to ensure that if the security of elliptic curve-based cryptography came into doubt (due to the emergence of a cryptographically relevant quantum computer or otherwise), then new Orchard funds could remain recoverable by a later protocol — as opposed to having to be burnt in order to avoid an unbounded balance violation",
          "answers": [
            "Support",
            "Oppose"
          ]
        }
      ]
    }
  ]
}""",
      seed:
          "stool rich together paddle together pool raccoon promote attitude peasant latin concert",
    );

    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('flutter_rust_bridge quickstart')),
        body: Center(child: Text(electionJSON)),
      ),
    );
  }
}
