import {Transformer} from '@atlaspack/plugin';
import {applyTokensPlugin} from '@atlaspack/rust';
import {validateSchema} from '@atlaspack/utils';
import path from 'path';

type AtlaskitTokensConfig = {
  shouldUseAutoFallback?: boolean;
  shouldForceAutoFallback?: boolean;
  forceAutoFallbackExemptions?: Array<string>;
  defaultTheme?: 'light' | 'legacy-light';
  tokenDataPath: string;
};

type LoadedConfig = {
  atlaskitTokens?: Required<Omit<AtlaskitTokensConfig, 'tokenDataPath'>> & {
    tokenDataPath: string;
  };
};

const CONFIG_SCHEMA = {
  type: 'object',
  properties: {
    atlaskitTokens: {
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
      required: ['tokenDataPath'],
      additionalProperties: false,
    },
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

    let loaded: LoadedConfig = {};
    if (conf && conf.contents) {
      validateSchema(
        {
          type: 'object',
          properties: CONFIG_SCHEMA.properties,
          additionalProperties: false,
        } as any,
        {data: conf.contents},
      );

      // @ts-expect-error TS2339
      if (conf.contents?.atlaskitTokens) {
        // @ts-expect-error TS2339
        const tokensConfig: AtlaskitTokensConfig = conf.contents.atlaskitTokens;
        const tokenDataPath = path.join(
          options.projectRoot,
          tokensConfig.tokenDataPath,
        );
        loaded.atlaskitTokens = {
          shouldUseAutoFallback: tokensConfig.shouldUseAutoFallback ?? true,
          shouldForceAutoFallback: tokensConfig.shouldForceAutoFallback ?? true,
          forceAutoFallbackExemptions:
            tokensConfig.forceAutoFallbackExemptions ?? [],
          defaultTheme: tokensConfig.defaultTheme ?? 'light',
          tokenDataPath,
        };
        config.invalidateOnFileChange(tokenDataPath);
      }
    }

    return loaded;
  },

  async transform({asset, options, config}) {
    const code = await asset.getCode();
    if (!code.includes('@atlaskit/tokens')) {
      return [asset];
    }

    const codeBuffer = Buffer.from(code);
    const tokensPath = config?.atlaskitTokens?.tokenDataPath;
    if (!tokensPath) {
      // If no config provided, just return asset unchanged.
      return [asset];
    }

    const compiledCode = (await applyTokensPlugin(
      codeBuffer,
      options.projectRoot,
      asset.filePath,
      asset.isSource,
      {
        tokens_path: tokensPath,
        should_use_auto_fallback: config.atlaskitTokens.shouldUseAutoFallback,
        should_force_auto_fallback:
          config.atlaskitTokens.shouldForceAutoFallback,
        force_auto_fallback_exemptions:
          config.atlaskitTokens.forceAutoFallbackExemptions,
        default_theme: config.atlaskitTokens.defaultTheme,
      },
    )) as Buffer;

    asset.setBuffer(compiledCode);
    return [asset];
  },
}) as Transformer<unknown>;
