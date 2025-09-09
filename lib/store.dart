import 'package:mobx/mobx.dart';

part 'store.g.dart';

var appStore = AppStore();

class AppStore = _AppStore with _$AppStore;

abstract class _AppStore with Store {
  @observable
  int counter = 0;

  @action
  void increment() {
    counter++;
  }
}
