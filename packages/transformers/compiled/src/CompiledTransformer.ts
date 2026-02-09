/**
 * Atlaspack transformer for Compiled CSS-in-JS.
 *
 * This is a direct port of @compiled/parcel-transformer to Atlaspack,
 * allowing users to transition without any change in functionality.
 */
import path, {join} from 'path';
import assert from 'assert';

import {transformAsync} from '@babel/core';
import generate from '@babel/generator';
import type {
  PluginOptions as BabelPluginOptions,
  Resolver,
} from '@compiled/babel-plugin';
import type {
  PluginOptions as BabelStripRuntimePluginOptions,
  BabelFileMetadata,
} from '@compiled/babel-plugin-strip-runtime';
import {Transformer} from '@atlaspack/plugin';
import SourceMap from '@atlaspack/source-map';
import {relativeUrl} from '@atlaspack/utils';
// eslint-disable-next-line import/no-extraneous-dependencies
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

/**
 * Module-level cache for resolver instances.
 * Key: resolver module path, Value: loaded resolver object
 *
 * We use a module-level cache because:
 * 1. The config returned from setup() must be serializable (no functions)
 * 2. Each worker process has its own cache (workers are separate Node.js processes)
 * 3. The resolver is loaded once per worker during setup(), avoiding FS operations during transform()
 */
const resolverCache = new Map<string, Resolver>();

/**
 * Loads and validates a custom resolver module.
 * The resolver can be specified as a module path string in the config.
 * This function resolves and loads it at setup time to avoid FS operations during transform.
 *
 * The loaded resolver is cached in the module-level cache, keyed by the resolver path.
 * In transform(), we retrieve the resolver from the cache instead of storing it in config
 * (since functions are not serializable).
 */
function loadResolver(resolverPath: string, projectRoot: string): Resolver {
  // Check cache first
  const cacheKey = `${projectRoot}:${resolverPath}`;
  const cached = resolverCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  try {
    const resolvedPath = require.resolve(resolverPath, {
      paths: [projectRoot],
    });
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const resolverModule = require(resolvedPath);

    let resolver: Resolver;
    // The module should export a resolveSync function directly or as a property
    if (typeof resolverModule.resolveSync === 'function') {
      resolver = resolverModule as Resolver;
    } else if (typeof resolverModule === 'function') {
      resolver = {resolveSync: resolverModule} as Resolver;
    } else {
      throw new Error(
        `Resolver module "${resolverPath}" does not export a valid resolveSync function`,
      );
    }

    // Cache the resolver for use in transform()
    resolverCache.set(cacheKey, resolver);
    return resolver;
  } catch (error) {
    throw new Error(
      `Failed to load resolver module "${resolverPath}": ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

/**
 * Gets the resolver for use in transform().
 * If a custom resolver path is specified, retrieves it from the cache.
 * Otherwise, creates a default resolver.
 */
function getResolver(config: Config): Resolver {
  if (config.resolverCacheKey) {
    const cached = resolverCache.get(config.resolverCacheKey);
    if (cached) {
      return cached;
    }
    // If not in cache (shouldn't happen in normal flow), fall through to default
  }
  return createDefaultResolver(config.compiledConfig);
}

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
  /**
   * Cache key for the resolver. If the config specifies a resolver as a string (module path),
   * it is resolved and loaded in setup() and cached in the module-level resolverCache.
   * This key is used to retrieve the resolver in transform().
   *
   * We use a cache key instead of the resolver object itself because the config must be
   * serializable (functions cannot be serialized across the Rust/JS boundary).
   */
  resolverCacheKey?: string;
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
    // tree reading browserslist package.json files on every transform, causing
    // cache bailouts for every file.
    // Internally, babel resolves configs relative to CWD. We do the same for parity.
    const cwd = '.';
    const absoluteCwd = path.resolve(cwd);
    let browserslistConfig: string[] | undefined = browserslist.loadConfig({
      path: absoluteCwd,
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

    // Pre-load the resolver module in setup() to avoid FS operations during transform().
    // The config can specify a resolver as a string (module path) which the Compiled babel
    // plugin would normally resolve via require() during transform. By loading it here,
    // we move those FS operations out of the transform phase, enabling caching.
    //
    // We store a cache key in the config (not the resolver itself) because the config must
    // be serializable. The resolver is stored in the module-level cache and retrieved in transform().
    let resolverCacheKey: string | undefined;
    if (typeof contents.resolver === 'string') {
      resolverCacheKey = `${options.projectRoot}:${contents.resolver}`;
      // Load the resolver now (during setup) to populate the cache
      // This ensures FS operations happen during setup, not during transform
      loadResolver(contents.resolver, options.projectRoot);
      // After loading, we strip the resolver string from contents so that it never appears in
      // compiledConfig. This prevents the Compiled babel plugin from seeing a string resolver
      // and doing require.resolve() during transform -- even if the resolver override in the
      // plugin options is omitted.
      // The resolverCacheKey preserves the string for cache key computation.
      delete contents.resolver;
    }

    return {
      config: {
        compiledConfig: contents,
        mode: options.mode,
        projectRoot: options.projectRoot,
        resolverCacheKey,
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
            // Use the pre-loaded resolver from setup(), or create a default one.
            // The resolver is retrieved from the module-level cache using the cache key.
            // By passing the resolver object (not a string), we avoid the Compiled
            // babel plugin doing require.resolve() during transform, which would
            // cause FS operations and cache bailouts.
            resolver: getResolver(config),
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
