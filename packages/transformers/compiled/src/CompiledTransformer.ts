/**
 * Atlaspack transformer for Compiled CSS-in-JS.
 *
 * This is a direct port of @compiled/parcel-transformer to Atlaspack,
 * allowing users to transition without any change in functionality.
 */
import {join} from 'path';
import assert from 'assert';

import {transformAsync} from '@babel/core';
import generate from '@babel/generator';
import type {PluginOptions as BabelPluginOptions} from '@compiled/babel-plugin';
import type {
  PluginOptions as BabelStripRuntimePluginOptions,
  BabelFileMetadata,
} from '@compiled/babel-plugin-strip-runtime';
import {Transformer} from '@atlaspack/plugin';
import SourceMap from '@atlaspack/source-map';
import {relativeUrl} from '@atlaspack/utils';
import browserslist from 'browserslist';

import type {CompiledTransformerOpts} from './types';
import {
  createDefaultResolver,
  DEFAULT_IMPORT_SOURCES,
  toBoolean,
} from './utils';
import {BuildMode} from '@atlaspack/types';
import CompiledBabelPlugin from '@compiled/babel-plugin';
import CompiledBabelPluginStripRuntime from '@compiled/babel-plugin-strip-runtime';
// @ts-expect-error no declaration file
// eslint-disable-next-line import/no-extraneous-dependencies
import BabelPluginSyntaxJsx from '@babel/plugin-syntax-jsx';
// @ts-expect-error no declaration file
// eslint-disable-next-line import/no-extraneous-dependencies
import BabelPluginSyntaxTypescript from '@babel/plugin-syntax-typescript';

const configFiles = [
  '.compiledcssrc',
  '.compiledcssrc.json',
  'compiledcss.js',
  'compiledcss.config.js',
];

const packageKey = '@atlaspack/transformer-compiled';

interface Config {
  compiledConfig: CompiledTransformerOpts;
  mode: BuildMode;
  projectRoot: string;
  browserslist?:
    | Array<string>
    | {
        [key: string]: Array<string>;
      };
}

/**
 * Atlaspack Compiled transformer.
 */
export default new Transformer<Config>({
  async setup({config, options}) {
    const conf = await config.getConfigFrom<CompiledTransformerOpts>(
      join(options.projectRoot, 'index'),
      configFiles,
      {
        packageKey,
      },
    );

    const contents: CompiledTransformerOpts = {
      extract: false,
      importReact: true,
      ssr: false,
      importSources: DEFAULT_IMPORT_SOURCES,
    };

    // Pre-load the browserslist config during setup().
    // If we don't do this, calling transformAsync() will cause Babel's
    // @babel/helper-compilation-targets to walk up the directory
    // tree reading package.json files on every transform, causing
    // cache bailouts for every file.
    let browserslistConfig: string[] | undefined = browserslist.loadConfig({
      path: options.projectRoot,
    });

    if (conf) {
      if (conf.filePath.endsWith('.js')) {
        config.invalidateOnStartup();
      }

      // Use `classNameCompressionMapFilePath` to get classNameCompressionMap
      // Note `classNameCompressionMap` and `classNameCompressionMapFilePath` are mutually exclusive.
      // If both are provided, classNameCompressionMap takes precedence.
      if (
        !conf.contents.classNameCompressionMap &&
        conf.contents.classNameCompressionMapFilePath
      ) {
        // Use `getConfigFrom` from Atlaspack so the contents are cached at `.parcel-cache`
        const configClassNameCompressionMap = await config.getConfigFrom(
          join(options.projectRoot, 'index'),
          [conf.contents.classNameCompressionMapFilePath],
          {
            packageKey,
          },
        );

        if (configClassNameCompressionMap?.contents) {
          Object.assign(contents, {
            classNameCompressionMap: configClassNameCompressionMap?.contents,
          });
        }
      }

      Object.assign(contents, conf.contents);

      contents.importSources = [
        ...DEFAULT_IMPORT_SOURCES,
        ...(contents.importSources ?? []),
      ];
    }

    // When transformerBabelPlugins is configured, we cannot cache transformer results
    // because these plugins are loaded dynamically by string name and the dev dep scanner
    // cannot track their dependencies for cache invalidation.
    const hasExternalBabelPlugins =
      contents.transformerBabelPlugins &&
      contents.transformerBabelPlugins.length > 0;

    return {
      config: {
        compiledConfig: contents,
        mode: options.mode,
        projectRoot: options.projectRoot,
        browserslist: browserslistConfig,
      },
      conditions: {
        codeMatch: contents.importSources,
      },
      env: [
        // TODO revisit this list, since we may have added variables in here that were actually enumarated rather than accessed directly
        'BABEL_ENV',
        'BABEL_SHOW_CONFIG_FOR',
        'BROWSERSLIST',
        'BROWSERSLIST_CONFIG',
        'BROWSERSLIST_DISABLE_CACHE',
        'BROWSERSLIST_ENV',
        'BROWSERSLIST_IGNORE_OLD_DATA',
        'BABEL_TYPES_8_BREAKING',
        'BROWSERSLIST_ROOT_PATH',
        'BROWSERSLIST_STATS',
        'AUTOPREFIXER',
        'AUTOPREFIXER_GRID',
        'TEST_PKG_VERSION',
        'FORCE_COLOR',
        'DEBUG',
        'NODE_DEBUG',
        'CI',
        'COLORTERM',
        'TERM',
      ],
      disableCache: hasExternalBabelPlugins,
    };
  },

  async transform({asset, config}) {
    if (
      config.compiledConfig.extract &&
      config.compiledConfig.classHashPrefix
    ) {
      throw new Error(
        '`@atlaspack/transformer-compiled` is mixing `extract: true` and `classHashPrefix` options, which will not supported and will result in bundle size bloat.',
      );
    }

    // Disable stylesheet extraction locally due to https://github.com/atlassian-labs/compiled/issues/1306
    const extract =
      config.compiledConfig.extract && config.mode !== 'development';
    if (!asset.isSource && !extract) {
      // Only parse source (pre-built code should already have been baked) or if stylesheet extraction is enabled
      return [asset];
    }

    const code = await asset.getCode();
    if (
      // If neither Compiled (default) nor any of the additional import sources are found in the code, we bail out.
      config.compiledConfig.importSources.every(
        (importSource) => !code.includes(importSource),
      )
    ) {
      // We only want to parse files that are actually using Compiled.
      // For everything else we bail out.
      return [asset];
    }
    if (code.includes('/* COMPILED_TRANSFORMED_ASSET */')) {
      // If we're dealing with a pre-transformed asset, we bail out to avoid performing the expensive parse operation.
      // We add this marker to the code to indicate that the asset has already been transformed.
      return [asset];
    }

    // Disable stylesheet extraction locally due to https://github.com/atlassian-labs/compiled/issues/1306
    const includedFiles: string[] = [];

    const result = await transformAsync(code, {
      code: false,
      ast: true,
      filename: asset.filePath,
      babelrc: false,
      configFile: false,
      // Disable browserslistConfigFile because we pass the browserslist config via the targets option
      // to prevent FS reads while resolving browserslist config.
      browserslistConfigFile: false,
      targets: config.browserslist ?? {},
      sourceMaps: !!asset.env.sourceMap,
      compact: false,
      plugins: [
        BabelPluginSyntaxJsx,
        [BabelPluginSyntaxTypescript, {isTSX: true}],
        ...(config.compiledConfig.transformerBabelPlugins ?? []),
        asset.isSource && [
          CompiledBabelPlugin,
          {
            ...config.compiledConfig,
            importSources: config.compiledConfig.importSources,
            classNameCompressionMap:
              config.compiledConfig.extract &&
              config.compiledConfig.classNameCompressionMap,
            onIncludedFiles: (files: string[]) => includedFiles.push(...files),
            resolver: config.compiledConfig.resolver
              ? config.compiledConfig.resolver
              : createDefaultResolver(config.compiledConfig),
            cache: false,
          } as BabelPluginOptions,
        ],
        extract && [
          CompiledBabelPluginStripRuntime,
          {
            compiledRequireExclude: true,
            extractStylesToDirectory:
              config.compiledConfig.extractStylesToDirectory,
          } as BabelStripRuntimePluginOptions,
        ],
      ].filter(toBoolean),
      caller: {
        name: 'compiled',
      },
    });

    includedFiles.forEach((file) => {
      // Included files are those which have been statically evaluated into this asset.
      // This tells atlaspack that if any of those files change this asset should be transformed
      // again.
      asset.invalidateOnFileChange(file);
    });

    if (extract) {
      // Store styleRules to asset.meta to be used by @atlaspack/optimizer-compiled
      const metadata = result?.metadata as BabelFileMetadata;
      asset.meta.styleRules = [
        ...((asset.meta?.styleRules as string[] | undefined) ?? []),
        ...(metadata.styleRules ?? []),
      ];
    }

    const originalSourceMap = await asset.getMap();
    const sourceFileName: string = relativeUrl(
      config.projectRoot,
      asset.filePath,
    );

    assert(result?.ast, 'Babel transform returned no AST');

    const {code: generatedCode, rawMappings} = generate(result.ast.program, {
      sourceFileName,
      sourceMaps: !!asset.env.sourceMap,
      comments: true,
    }) as {
      code: string;
      rawMappings?: Array<{
        generated: {line: number; column: number};
        source: string;
        original: {line: number; column: number};
        name?: string;
      }>;
    };

    asset.setCode(generatedCode);

    const map = new SourceMap(config.projectRoot);
    if (rawMappings) {
      map.addIndexedMappings(rawMappings);
    }

    if (originalSourceMap) {
      // The babel AST already contains the correct mappings, but not the source contents.
      // We need to copy over the source contents from the original map.
      const sourcesContent = originalSourceMap.getSourcesContentMap();
      for (const filePath in sourcesContent) {
        const content = sourcesContent[filePath];
        if (content != null) {
          map.setSourceContent(filePath, content);
        }
      }
    }

    return [asset];
  },
});
