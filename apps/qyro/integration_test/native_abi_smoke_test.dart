import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:qyro/ffi/qyro_native_api.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('loads qyro_ffi and reads QYRO/1 inside Android runtime',
      (tester) async {
    final api = QyroNativeApi.openDefault();

    expect(api.protocolVersion(), 'QYRO/1');
  });
}
