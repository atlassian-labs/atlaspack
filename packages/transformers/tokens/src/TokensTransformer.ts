import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {applyTokensPlugin, TokensPluginResult} from '@atlaspack/rust';
import SourceMap from '@atlaspack/source-map';
import {loadTokensConfig} from '@atlaspack/transformer-js';

export default new Transformer({
  // eslint-disable-next-line require-await
  async loadConfig({config, options}) {
    if (
      !getFeatureFlag('enableTokensTransformer') ||
      getFeatureFlag('coreTokensAndCompiledCssInJsTransform')
    ) {
      return undefined;
    }

    return loadTokensConfig(config, options);
  },

  async transform({asset, options, config}) {
    if (
      !getFeatureFlag('enableTokensTransformer') ||
      getFeatureFlag('coreTokensAndCompiledCssInJsTransform')
    ) {
      return [asset];
    }

    const [code, originalMap] = await Promise.all([
      asset.getCode(),
      asset.getMap(),
    ]);

    if (!code.includes('@atlaskit/tokens')) {
      return [asset];
    }

    const codeBuffer = Buffer.from(code);
    if (!config) {
      // If no config provided, just return asset unchanged.
      return [asset];
    }

    const result = await (applyTokensPlugin(codeBuffer, {
      filename: asset.filePath,
      projectRoot: options.projectRoot,
      isSource: asset.isSource,
      sourceMaps: !!asset.env.sourceMap,
      tokensOptions: {
        ...config,
      },
    }) as Promise<TokensPluginResult>);

    // Ensure this transform is invalidated when the token data changes
    asset.invalidateOnFileChange(config.tokenDataPath);

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
