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
import {DEFAULT_IMPORT_SOURCES, toBoolean} from '@compiled/utils';
import {Transformer} from '@atlaspack/plugin';
import SourceMap from '@atlaspack/source-map';
import {relativeUrl} from '@atlaspack/utils';

import type {CompiledTransformerOpts} from './types';
import {createDefaultResolver} from './utils';
import {BuildMode} from '@atlaspack/types';
import CompiledBabelPlugin from '@compiled/babel-plugin';
import CompiledBabelPluginStripRuntime from '@compiled/babel-plugin-strip-runtime';
// @ts-expect-error no declaration file
import BabelPluginSyntaxJsx from '@babel/plugin-syntax-jsx';
// @ts-expect-error no declaration file
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
    };

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
    }

    let importSourceMatches = [
      ...DEFAULT_IMPORT_SOURCES,
      ...(contents.importSources || []),
    ];

    return {
      config: {
        compiledConfig: contents,
        mode: options.mode,
        projectRoot: options.projectRoot,
      },
      conditions: {
        codeMatch: importSourceMatches,
      },
      env: ['BABEL_ENV', 'BABEL_SHOW_CONFIG_FOR'],
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
      [
        ...DEFAULT_IMPORT_SOURCES,
        ...(config.compiledConfig.importSources || []),
      ].every((importSource) => !code.includes(importSource))
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
      sourceMaps: !!asset.env.sourceMap,
      compact: false,
      plugins: [
        BabelPluginSyntaxJsx,
        [BabelPluginSyntaxTypescript, {isTSX: true}],
        asset.isSource && [
          CompiledBabelPlugin,
          {
            ...config,
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
