import 'dart:io';

import '../lib/branding_generator.dart';

void main(List<String> arguments) {
  try {
    final options = _Options.parse(arguments);
    final script = File.fromUri(Platform.script);
    final repositoryRoot = script.parent.parent.parent.parent;
    final localConfig = File(
      '${repositoryRoot.path}${Platform.pathSeparator}config'
      '${Platform.pathSeparator}branding.json',
    );
    final fallbackConfig = File(
      '${repositoryRoot.path}${Platform.pathSeparator}config'
      '${Platform.pathSeparator}branding.example.json',
    );
    final input = options.inputPath == null
        ? (localConfig.existsSync() ? localConfig : fallbackConfig)
        : File(options.inputPath!);
    final output = File(
      options.outputPath ??
          '${repositoryRoot.path}${Platform.pathSeparator}apps'
              '${Platform.pathSeparator}qyro${Platform.pathSeparator}lib'
              '${Platform.pathSeparator}generated'
              '${Platform.pathSeparator}branding.g.dart',
    );

    if (!input.existsSync()) {
      throw BrandingValidationException(
        'Branding input does not exist: ${input.path}',
      );
    }

    final result = const BrandingGenerator().generate(
      input.readAsStringSync(),
      requireFinal: options.requireFinal,
    );

    if (options.check) {
      final current = output.existsSync() ? output.readAsStringSync() : null;
      if (current != result.dartSource) {
        throw const BrandingValidationException(
          'Generated branding is stale; run the branding generator',
        );
      }
      stdout.writeln('[PASS] Generated branding is current');
      return;
    }

    output.parent.createSync(recursive: true);
    output.writeAsStringSync(result.dartSource);
    stdout.writeln(
      '[PASS] Generated ${output.path}'
      '${result.isProvisional ? ' (provisional)' : ''}',
    );
  } on BrandingValidationException catch (error) {
    stderr.writeln('[BLOCKER] ${error.message}');
    exitCode = 1;
  } on FormatException catch (error) {
    stderr.writeln('[BLOCKER] ${error.message}');
    exitCode = 64;
  }
}

final class _Options {
  const _Options({
    required this.check,
    required this.requireFinal,
    this.inputPath,
    this.outputPath,
  });

  final bool check;
  final bool requireFinal;
  final String? inputPath;
  final String? outputPath;

  static _Options parse(List<String> arguments) {
    var check = false;
    var requireFinal = false;
    String? inputPath;
    String? outputPath;

    for (final argument in arguments) {
      if (argument == '--check') {
        check = true;
      } else if (argument == '--require-final') {
        requireFinal = true;
      } else if (argument.startsWith('--input=')) {
        inputPath = argument.substring('--input='.length);
      } else if (argument.startsWith('--output=')) {
        outputPath = argument.substring('--output='.length);
      } else {
        throw FormatException('Unknown argument: $argument');
      }
    }

    return _Options(
      check: check,
      requireFinal: requireFinal,
      inputPath: inputPath,
      outputPath: outputPath,
    );
  }
}
