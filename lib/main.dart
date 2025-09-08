import 'package:flutter/material.dart';
import 'package:zcv/router.dart';
import 'package:zcv/src/rust/api/init.dart';
import 'package:zcv/src/rust/api/prop.dart';
import 'package:zcv/src/rust/frb_generated.dart';
import 'package:path_provider/path_provider.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final path = await getApplicationDocumentsDirectory();
  await setDbPath(dir: path.path, name: "zcv.db");
  await setLwd(lwd: "https://zec.rocks");
  await putProp(key: "test", value: "true");
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp.router(
      routerConfig: router,
      debugShowCheckedModeBanner: false,
    );
  }
}
