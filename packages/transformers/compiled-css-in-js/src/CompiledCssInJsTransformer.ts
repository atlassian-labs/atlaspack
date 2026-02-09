import type {SourceLocation} from '@atlaspack/types';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {
  applyCompiledCssInJsPlugin,
  CompiledCssInJsPluginResult,
  hashCode,
  isSafeFromJs,
} from '@atlaspack/rust';
import SourceMap from '@atlaspack/source-map';
import type {Diagnostic} from '@atlaspack/diagnostic';
import ThrowableDiagnostic, {
  convertSourceLocationToHighlight,
} from '@atlaspack/diagnostic';
import {remapSourceLocation} from '@atlaspack/utils';

import {loadCompiledCssInJsConfig} from '@atlaspack/transformer-js';

export default new Transformer({
  // eslint-disable-next-line require-await
  async loadConfig({config, options}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return undefined;
    }

    return loadCompiledCssInJsConfig(config, options);
  },
  async transform({asset, options, config, logger}) {
    if (!getFeatureFlag('compiledCssInJsTransformer') || !config) {
      return [asset];
    }

    if (!asset.isSource && !config.extract) {
      return [asset];
    }

    const code = await asset.getCode();

    // If neither Compiled (default) nor any of the additional import sources are found in the code, we bail out.
    if (
      config.importSources.every((importSource) => !code.includes(importSource))
    ) {
      return [asset];
    }

    if (
      getFeatureFlag('compiledCssInJsTransformer') &&
      (config?.unsafeReportSafeAssetsForMigration ||
        config?.unsafeUseSafeAssets)
    ) {
      asset.meta.compiledCodeHash ??= hashCode(code);
    }

    if (config?.unsafeUseSafeAssets) {
      if (!config.configPath) {
        throw new Error(
          'configPath is required when unsafeUseSafeAssets is enabled',
        );
      }

      asset.meta.useRustCompiledTransform ??= isSafeFromJs(
        asset.meta.compiledCodeHash as string,
        config.configPath,
      );
    }

    if (getFeatureFlag('coreTokensAndCompiledCssInJsTransform')) {
      // Skipping if we're using the compiled CSS in JS transform from the core pass
      return [asset];
    }

    if (config.unsafeUseSafeAssets && !asset.meta.useRustCompiledTransform) {
      // Fallback to the legacy transform if we know the asset is not safe
      return [asset];
    }

    const mapPromise = asset.getMap();
    let originalMap: SourceMap | null | undefined;
    const ensureOriginalMap = async () => {
      if (originalMap === undefined) {
        originalMap = await mapPromise;
      }

      return originalMap;
    };

    const codeBuffer = Buffer.from(code);

    const result = (await applyCompiledCssInJsPlugin(codeBuffer, {
      filename: asset.filePath,
      projectRoot: options.projectRoot,
      isSource: asset.isSource,
      sourceMaps: !!asset.env.sourceMap,
      config,
    })) as CompiledCssInJsPluginResult;

    if (result.diagnostics?.length > 0) {
      const diagnostics = result.diagnostics ?? [];
      asset.meta.compiledCssDiagnostics = JSON.parse(
        JSON.stringify(diagnostics),
      );

      const original = await ensureOriginalMap();
      type PluginDiagnostic = (typeof diagnostics)[number];
      type PluginCodeHighlight = NonNullable<
        PluginDiagnostic['codeHighlights']
      >[number];
      type PluginSourceLocation = PluginCodeHighlight['loc'];

      const convertLoc = (loc: PluginSourceLocation): SourceLocation => {
        let location: SourceLocation = {
          filePath: asset.filePath,
          start: {
            line: loc.startLine + Number(asset.meta.startLine ?? 1) - 1,
            column: loc.startCol,
          },
          end: {
            line: loc.endLine + Number(asset.meta.startLine ?? 1) - 1,
            column: loc.endCol,
          },
        };

        if (original) {
          location = remapSourceLocation(
            location,
            original,
            options.projectRoot,
          );
        }

        return location;
      };

      const convertDiagnostic = (diagnostic: PluginDiagnostic): Diagnostic => {
        const codeHighlights = (diagnostic.codeHighlights ?? []).map(
          (highlight: PluginCodeHighlight) =>
            convertSourceLocationToHighlight(
              convertLoc(highlight.loc),
              highlight.message ?? undefined,
            ),
        );

        const converted: Diagnostic = {
          message: diagnostic.message,
          codeFrames: [
            {
              filePath: asset.filePath,
              codeHighlights,
            },
          ],
          hints: diagnostic.hints,
        };

        if (diagnostic.documentationUrl) {
          converted.documentationURL = diagnostic.documentationUrl;
        }

        if (diagnostic.showEnvironment && asset.env.loc) {
          converted.codeFrames?.push({
            filePath: asset.env.loc.filePath,
            codeHighlights: [
              convertSourceLocationToHighlight(
                asset.env.loc,
                'The environment was originally created here',
              ),
            ],
          });
        }

        return converted;
      };

      const errors = diagnostics.filter(
        (diagnostic) =>
          diagnostic.severity === 'Error' ||
          (diagnostic.severity === 'SourceError' && asset.isSource),
      );
      if (errors.length > 0) {
        if (
          config.unsafeUseSafeAssets ||
          config.unsafeReportSafeAssetsForMigration
        ) {
          for (const error of errors) {
            logger.warn(convertDiagnostic(error));
          }
        } else {
          throw new ThrowableDiagnostic({
            diagnostic: errors.map(convertDiagnostic),
          });
        }
      }

      const warnings = diagnostics.filter(
        (diagnostic) =>
          diagnostic.severity === 'Warning' ||
          (diagnostic.severity === 'SourceError' && !asset.isSource),
      );
      if (warnings.length > 0) {
        for (const warning of warnings) {
          logger.warn(convertDiagnostic(warning));
        }
      }
    }

    if (config.unsafeReportSafeAssetsForMigration) {
      // We need to run the transform without returning the result, so we can report the safe assets
      asset.meta.swcStyleRules = result.styleRules;
      asset.meta.compiledCssDiagnostics = result.diagnostics.map(
        (d) => d.message,
      );
      asset.meta.compiledBailOut = result.bailOut;

      return [asset];
    }

    if (result.bailOut) {
      // Bail out if the transform failed
      return [asset];
    }

    // Handle sourcemap merging if sourcemap is generated
    if (result.map != null) {
      let map = new SourceMap(options.projectRoot);
      map.addVLQMap(JSON.parse(result.map));
      const original = await ensureOriginalMap();
      if (original) {
        map.extends(original);
      }
      asset.setMap(map);
    }

    // Rather then setting this as a buffer we set it as a string, since most of the following
    // plugins will call `getCode`, this avoids repeatedly converting the buffer to a string.
    asset.setCode(result.code);

    // Add styleRules to the asset
    if (config.extract) {
      // Note: we only set styleRules if extract is true, this is because we will duplicate style rules on the client.
      // This will cause undefined behaviour because the style rules will race for specificity based on ordering
      asset.meta.styleRules = [
        ...((asset.meta.styleRules as string[]) || []),
        ...result.styleRules,
      ];
    }

    // Add File dependencies for any imported style files
    if (result.includedFiles && result.includedFiles.length > 0) {
      for (const includedFile of result.includedFiles) {
        asset.addIncludedFile({
          filePath: includedFile,
        });
      }
    }

    return [asset];
  },
});
