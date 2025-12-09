import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {isSafeFromJs, hashCode} from '@atlaspack/rust/index';
import {applyTokensPlugin, TokensPluginResult} from '@atlaspack/rust';
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

  async transform({asset, options, config}) {
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

    const result = await (applyTokensPlugin(codeBuffer, {
      filename: asset.filePath,
      projectRoot: options.projectRoot,
      isSource: asset.isSource,
      sourceMaps: !!asset.env.sourceMap,
      tokensOptions: {
        ...config.tokensConfig,
      },
    }) as Promise<TokensPluginResult>);

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
