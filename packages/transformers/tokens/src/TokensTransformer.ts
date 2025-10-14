import {encodeJSONKeyComponent} from '@atlaspack/diagnostic';
import {Transformer} from '@atlaspack/plugin';
import {applyTokensPlugin} from '@atlaspack/rust';
import {validateSchema} from '@atlaspack/utils';
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
          // FIXME
          source: await options.inputFS.readFile(conf.filePath, 'utf8'),
          filePath: conf.filePath,
          prependKey: `/${encodeJSONKeyComponent('@atlaspack/transformer-tokens')}`,
        },
        // FIXME
        '@atlaspack/transformer-tokens',
        'Invalid config for @atlaspack/transformer-js',
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
      config.invalidateOnFileChange(resolvedConfig.tokenDataPath);
      return resolvedConfig;
    }
  },

  async transform({asset, options, config}) {
    const code = await asset.getCode();
    if (!code.includes('@atlaskit/tokens')) {
      return [asset];
    }

    const codeBuffer = Buffer.from(code);
    if (!config) {
      // If no config provided, just return asset unchanged.
      return [asset];
    }

    const compiledCode = await applyTokensPlugin(
      codeBuffer,
      options.projectRoot,
      asset.filePath,
      asset.isSource,
      {
        tokensPath: config.tokenDataPath,
        shouldUseAutoFallback: config.shouldUseAutoFallback,
        shouldForceAutoFallback: config.shouldForceAutoFallback,
        forceAutoFallbackExemptions: config.forceAutoFallbackExemptions,
        defaultTheme: config.defaultTheme,
      },
    );

    // Rather then setting this as a buffer we set it as a string, since most of the following
    // plugins will call `getCode`, this avoids repeatedly converting the buffer to a string.
    asset.setCode(compiledCode);
    return [asset];
  },
}) as Transformer<unknown>;
