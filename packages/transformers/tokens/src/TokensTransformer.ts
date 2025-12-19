import {
  encodeJSONKeyComponent,
  default as ThrowableDiagnostic,
  convertSourceLocationToHighlight,
  type Diagnostic,
} from '@atlaspack/diagnostic';
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
          source: () => options.inputFS.readFileSync(conf.filePath, 'utf8'),
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

    const codeBuffer = Buffer.from(code);
    if (!config) {
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
          ...config,
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
