import {
  encodeJSONKeyComponent,
  default as ThrowableDiagnostic,
  convertSourceLocationToHighlight,
  type Diagnostic,
} from '@atlaspack/diagnostic';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {
  applyTokensPlugin,
  hashCode,
  isSafeFromJs,
  TokensPluginResult,
} from '@atlaspack/rust';
import SourceMap from '@atlaspack/source-map';
import {
  loadCompiledCssInJsConfig,
  loadTokensConfig,
} from '@atlaspack/transformer-js';

export default new Transformer({
  // eslint-disable-next-line require-await
  async loadConfig({config, options}) {
    if (!getFeatureFlag('enableTokensTransformer')) {
      return undefined;
    }

    return {
      tokensConfig: await loadTokensConfig(config, options),
      compiledCssInJsConfig: await loadCompiledCssInJsConfig(config, options),
    };
  },

  async transform({asset, options, config, logger}) {
    if (!getFeatureFlag('enableTokensTransformer')) {
      return [asset];
    }

    const [code, originalMap] = await Promise.all([
      asset.getCode(),
      asset.getMap(),
    ]);

    if (!code.includes('@atlaskit/tokens')) {
      return [asset];
    }

    if (getFeatureFlag('compiledCssInJsTransformer')) {
      if (
        config?.compiledCssInJsConfig.unsafeReportSafeAssetsForMigration ||
        config?.compiledCssInJsConfig?.unsafeUseSafeAssets
      ) {
        asset.meta.compiledCodeHash ??= hashCode(code);
      }

      if (config?.compiledCssInJsConfig?.unsafeUseSafeAssets) {
        if (!config.compiledCssInJsConfig.configPath) {
          throw new Error(
            'configPath is required when unsafeUseSafeAssets is enabled',
          );
        }

        asset.meta.useRustCompiledTransform ??= isSafeFromJs(
          asset.meta.compiledCodeHash as string,
          config.compiledCssInJsConfig.configPath,
        );

        if (asset.meta.useRustCompiledTransform) {
          return [asset];
        }
      }
    }

    const codeBuffer = Buffer.from(code);
    if (!config?.tokensConfig) {
      // If no config provided, just return asset unchanged.
      return [asset];
    }

    const result = await (
      applyTokensPlugin(codeBuffer, {
        filename: asset.filePath,
        projectRoot: options.projectRoot,
        isSource: asset.isSource,
        sourceMaps: !!asset.env.sourceMap,
        tokensOptions: {
          ...config.tokensConfig,
        },
      }) as Promise<TokensPluginResult>
    ).catch((error) => {
      // Re-throw with context about which file failed
      throw new Error(
        `Failed to transform tokens in ${asset.filePath}: ${error.message || error}`,
      );
    });

    // Check for diagnostics and convert them to proper Diagnostic objects with code frames
    if (result.diagnostics && result.diagnostics.length > 0) {
      const convertDiagnostic = (diagnostic: any): Diagnostic => {
        const codeHighlights = diagnostic.code_highlights?.map(
          (highlight: any) =>
            convertSourceLocationToHighlight(
              {
                start: {
                  line: highlight.loc.start_line,
                  column: highlight.loc.start_col,
                },
                end: {
                  line: highlight.loc.end_line,
                  column: highlight.loc.end_col,
                },
              },
              highlight.message ?? undefined,
            ),
        );

        const res: Diagnostic = {
          message: diagnostic.message,
          codeFrames: [
            {
              filePath: asset.filePath,
              code: code,
              codeHighlights: codeHighlights ?? [],
            },
          ],
          hints: diagnostic.hints,
        };

        if (diagnostic.documentation_url) {
          res.documentationURL = diagnostic.documentation_url;
        }

        return res;
      };

      const errors = result.diagnostics.filter(
        (d: any) =>
          d.severity === 'Error' ||
          (d.severity === 'SourceError' && asset.isSource),
      );

      if (errors.length > 0) {
        throw new ThrowableDiagnostic({
          diagnostic: errors.map(convertDiagnostic),
        });
      }

      // Log warnings
      const warnings = result.diagnostics.filter(
        (d: any) =>
          d.severity === 'Warning' ||
          (d.severity === 'SourceError' && !asset.isSource),
      );
      if (warnings.length > 0) {
        logger.warn(warnings.map(convertDiagnostic));
      }
    }

    // Ensure this transform is invalidated when the token data changes
    asset.invalidateOnFileChange(config.tokensConfig.tokenDataPath);

    // Handle sourcemap merging if sourcemap is generated
    if (result.map != null) {
      let map = new SourceMap(options.projectRoot);
      map.addVLQMap(JSON.parse(result.map));
      if (originalMap) {
        map.extends(originalMap);
      }
      asset.setMap(map);
    }

    asset.setCode(result.code);
    return [asset];
  },
}) as Transformer<unknown>;
