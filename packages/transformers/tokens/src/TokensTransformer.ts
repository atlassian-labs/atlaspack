import {encodeJSONKeyComponent} from '@atlaspack/diagnostic';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {applyTokensPlugin, TokensPluginResult} from '@atlaspack/rust';
import {validateSchema} from '@atlaspack/utils';
import SourceMap from '@atlaspack/source-map';
import path from 'path';

type AtlaskitTokensConfigPartial = {
  shouldUseAutoFallback?: boolean;
  shouldForceAutoFallback?: boolean;
  forceAutoFallbackExemptions?: Array<string>;
  defaultTheme?: 'light' | 'legacy-light';
  tokenDataPath: string;
};

type AtlaskitTokensConfig = Required<AtlaskitTokensConfigPartial>;

const CONFIG_SCHEMA = {
  type: 'object',
  properties: {
    shouldUseAutoFallback: {type: 'boolean'},
    shouldForceAutoFallback: {type: 'boolean'},
    forceAutoFallbackExemptions: {
      type: 'array',
      items: {type: 'string'},
    },
    defaultTheme: {type: 'string', enum: ['light', 'legacy-light']},
    tokenDataPath: {type: 'string'},
  },
  additionalProperties: false,
} as const;

export default new Transformer({
  async loadConfig({config, options}) {
    const conf = await config.getConfigFrom(
      options.projectRoot + '/index',
      [],
      {
        packageKey: '@atlaspack/transformer-tokens',
      },
    );

    if (conf && conf.contents) {
      validateSchema.diagnostic(
        CONFIG_SCHEMA,
        {
          data: conf.contents,
          source: getFeatureFlag('schemaValidationDeferSourceLoading')
            ? () => options.inputFS.readFileSync(conf.filePath, 'utf8')
            : await options.inputFS.readFile(conf.filePath, 'utf8'),
          filePath: conf.filePath,
          prependKey: `/${encodeJSONKeyComponent('@atlaspack/transformer-tokens')}`,
        },
        '@atlaspack/transformer-tokens',
        'Invalid config for @atlaspack/transformer-tokens',
      );

      // @ts-expect-error TS2339
      const tokensConfig: AtlaskitTokensConfigPartial = conf.contents;

      let resolvedConfig: AtlaskitTokensConfig = {
        shouldUseAutoFallback: tokensConfig.shouldUseAutoFallback ?? true,
        shouldForceAutoFallback: tokensConfig.shouldForceAutoFallback ?? true,
        forceAutoFallbackExemptions:
          tokensConfig.forceAutoFallbackExemptions ?? [],
        defaultTheme: tokensConfig.defaultTheme ?? 'light',
        tokenDataPath: path.join(
          options.projectRoot,
          tokensConfig.tokenDataPath,
        ),
      };
      return resolvedConfig;
    }
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
