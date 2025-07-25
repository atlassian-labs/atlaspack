// @flow strict-local
import type {
  BundleGraph,
  BundleGroup,
  Dependency,
  Environment,
  PluginOptions,
  NamedBundle,
  RuntimeAsset,
} from '@atlaspack/types';

import {Runtime} from '@atlaspack/plugin';
import {
  relativeBundlePath,
  validateSchema,
  type SchemaEntity,
} from '@atlaspack/utils';
import {encodeJSONKeyComponent} from '@atlaspack/diagnostic';
import path from 'path';
import nullthrows from 'nullthrows';
import {getFeatureFlag} from '@atlaspack/feature-flags';

// Used for as="" in preload/prefetch
const TYPE_TO_RESOURCE_PRIORITY = {
  css: 'style',
  js: 'script',
};

const BROWSER_PRELOAD_LOADER = './helpers/browser/preload-loader';
const BROWSER_PREFETCH_LOADER = './helpers/browser/prefetch-loader';

const LOADERS = {
  browser: {
    css: './helpers/browser/css-loader',
    html: './helpers/browser/html-loader',
    js: './helpers/browser/js-loader',
    wasm: './helpers/browser/wasm-loader',
    IMPORT_POLYFILL: './helpers/browser/import-polyfill',
  },
  worker: {
    js: './helpers/worker/js-loader',
    wasm: './helpers/worker/wasm-loader',
    IMPORT_POLYFILL: false,
  },
  node: {
    css: './helpers/node/css-loader',
    html: './helpers/node/html-loader',
    js: './helpers/node/js-loader',
    wasm: './helpers/node/wasm-loader',
    IMPORT_POLYFILL: null,
  },
};

function getLoaders(
  ctx: Environment,
): ?{[string]: string, IMPORT_POLYFILL: null | false | string, ...} {
  if (ctx.isWorker()) return LOADERS.worker;
  if (ctx.isBrowser()) return LOADERS.browser;
  if (ctx.isNode()) return LOADERS.node;
  return null;
}

// This cache should be invalidated if new dependencies get added to the bundle without the bundle objects changing
// This can happen when we reuse the BundleGraph between subsequent builds
let bundleDependencies = new WeakMap<
  NamedBundle,
  {|
    asyncDependencies: Array<Dependency>,
    conditionalDependencies: Array<Dependency>,
    otherDependencies: Array<Dependency>,
  |},
>();

type JSRuntimeConfig = {|
  splitManifestThreshold: number,
  domainSharding?: {|
    maxShards: number,
  |},
|};

let defaultConfig: JSRuntimeConfig = {
  splitManifestThreshold: 100000,
};

const CONFIG_SCHEMA: SchemaEntity = {
  type: 'object',
  properties: {
    splitManifestThreshold: {
      type: 'number',
    },
    domainSharding: {
      type: 'object',
      properties: {
        maxShards: {
          type: 'number',
        },
      },
      additionalProperties: false,
      required: ['maxShards'],
    },
  },
  additionalProperties: false,
};

export default (new Runtime({
  async loadConfig({config, options}): Promise<JSRuntimeConfig> {
    let packageKey = '@atlaspack/runtime-js';
    let conf = await config.getConfig<JSRuntimeConfig>([], {
      packageKey,
    });

    if (!conf) {
      return defaultConfig;
    }
    validateSchema.diagnostic(
      CONFIG_SCHEMA,
      {
        data: conf?.contents,
        source: await options.inputFS.readFile(conf.filePath, 'utf8'),
        filePath: conf.filePath,
        prependKey: `/${encodeJSONKeyComponent(packageKey)}`,
      },
      packageKey,
      `Invalid config for ${packageKey}`,
    );

    return {
      ...defaultConfig,
      ...conf?.contents,
    };
  },
  apply({bundle, bundleGraph, options, config}) {
    // Dependency ids in code replaced with referenced bundle names
    // Loader runtime added for bundle groups that don't have a native loader (e.g. HTML/CSS/Worker - isURL?),
    // and which are not loaded by a parent bundle.
    // Loaders also added for modules that were moved to a separate bundle because they are a different type
    // (e.g. WASM, HTML). These should be preloaded prior to the bundle being executed. Replace the entry asset(s)
    // with the preload module.

    if (bundle.type !== 'js') {
      return;
    }

    let {asyncDependencies, conditionalDependencies, otherDependencies} =
      getDependencies(bundle);

    let assets = [];
    for (let dependency of asyncDependencies) {
      let resolved = bundleGraph.resolveAsyncDependency(dependency, bundle);
      if (resolved == null) {
        continue;
      }

      if (resolved.type === 'asset') {
        if (!bundle.env.shouldScopeHoist) {
          // If this bundle already has the asset this dependency references,
          // return a simple runtime of `Promise.resolve(internalRequire(assetId))`.
          // The linker handles this for scope-hoisting.

          const requireName = getFeatureFlag('hmrImprovements')
            ? 'parcelRequire'
            : 'module.bundle.root';

          assets.push({
            filePath: __filename,
            code: `module.exports = Promise.resolve(${requireName}(${JSON.stringify(
              bundleGraph.getAssetPublicId(resolved.value),
            )}))`,
            dependency,
            env: {sourceType: 'module'},
          });
        }
      } else {
        // Resolve the dependency to a bundle. If inline, export the dependency id,
        // which will be replaced with the contents of that bundle later.
        let referencedBundle = bundleGraph.getReferencedBundle(
          dependency,
          bundle,
        );
        if (referencedBundle?.bundleBehavior === 'inline') {
          assets.push({
            filePath: path.join(
              __dirname,
              `/bundles/${referencedBundle.id}.js`,
            ),
            code: `module.exports = Promise.resolve(${JSON.stringify(
              dependency.id,
            )});`,
            dependency,
            env: {sourceType: 'module'},
          });
          continue;
        }

        let loaderRuntime = getLoaderRuntime({
          bundle,
          dependency,
          bundleGraph,
          bundleGroup: resolved.value,
          options,
          shardingConfig: config.domainSharding,
        });

        if (loaderRuntime != null) {
          assets.push(loaderRuntime);
        }
      }
    }

    if (getFeatureFlag('conditionalBundlingApi')) {
      // For any conditions that are used in this bundle, we want to produce a runtime asset that is used to
      // select the correct dependency that condition maps to at runtime - the conditions in the bundle will then be
      // replaced with a reference to this asset to implement the selection.
      const conditions = bundleGraph.getConditionsForDependencies(
        conditionalDependencies,
        bundle,
      );
      for (const cond of conditions) {
        const requireName =
          getFeatureFlag('hmrImprovements') || bundle.env.shouldScopeHoist
            ? 'parcelRequire'
            : '__parcel__require__';

        const fallbackUrls = (cond) => {
          return `[${[...cond.ifTrueBundles, ...cond.ifFalseBundles]
            .map((target) => {
              let relativePathExpr = getRelativePathExpr(
                bundle,
                target,
                options,
              );
              return getAbsoluteUrlExpr(
                relativePathExpr,
                bundle,
                config.domainSharding,
              );
            })
            .join(',')}]`;
        };

        const shouldUseFallback =
          options.mode === 'development'
            ? getFeatureFlag('condbDevFallbackDev')
            : getFeatureFlag('condbDevFallbackProd');

        const loaderPath = `./helpers/conditional-loader${
          options.mode === 'development' ? '-dev' : ''
        }`;

        const ifTrue = `function (){return ${requireName}('${cond.ifTrueAssetId}')}`;
        const ifFalse = `function (){return ${requireName}('${cond.ifFalseAssetId}')}`;

        const assetCode = `module.exports = require('${loaderPath}')('${
          cond.key
        }', ${ifTrue}, ${ifFalse}${
          shouldUseFallback
            ? `, {loader: require('./helpers/browser/sync-js-loader'), urls: ${fallbackUrls(
                cond,
              )}}`
            : ''
        })`;

        assets.push({
          filePath: path.join(__dirname, `/conditions-${cond.publicId}.js`),
          code: assetCode,
          // This dependency is important, as it's the last symbol handled in scope hoisting.
          // That means that scope hoisting will use the module id for this asset to replace the symbol
          // (rather than the actual conditional deps)
          dependency: cond.ifFalseDependency,
          env: {sourceType: 'module'},
        });
      }
    }

    for (let dependency of otherDependencies) {
      // Resolve the dependency to a bundle. If inline, export the dependency id,
      // which will be replaced with the contents of that bundle later.
      let referencedBundle = bundleGraph.getReferencedBundle(
        dependency,
        bundle,
      );
      if (referencedBundle?.bundleBehavior === 'inline') {
        assets.push({
          filePath: path.join(__dirname, `/bundles/${referencedBundle.id}.js`),
          code: `module.exports = ${JSON.stringify(dependency.id)};`,
          dependency,
          env: {sourceType: 'module'},
        });
        continue;
      }

      // Otherwise, try to resolve the dependency to an external bundle group
      // and insert a URL to that bundle.
      let resolved = bundleGraph.resolveAsyncDependency(dependency, bundle);
      if (dependency.specifierType === 'url' && resolved == null) {
        // If a URL dependency was not able to be resolved, add a runtime that
        // exports the original specifier.
        assets.push({
          filePath: __filename,
          code: `module.exports = ${JSON.stringify(dependency.specifier)}`,
          dependency,
          env: {sourceType: 'module'},
        });
        continue;
      }

      if (resolved == null || resolved.type !== 'bundle_group') {
        continue;
      }

      let bundleGroup = resolved.value;
      let mainBundle = nullthrows(
        bundleGraph.getBundlesInBundleGroup(bundleGroup).find((b) => {
          let entries = b.getEntryAssets();
          return entries.some((e) => bundleGroup.entryAssetId === e.id);
        }),
      );

      // Skip URL runtimes for library builds. This is handled in packaging so that
      // the url is inlined and statically analyzable.
      if (bundle.env.isLibrary && mainBundle.bundleBehavior !== 'isolated') {
        continue;
      }

      // URL dependency or not, fall back to including a runtime that exports the url
      assets.push(
        getURLRuntime(
          dependency,
          bundle,
          mainBundle,
          options,
          // $FlowFixMe
          config.domainSharding,
        ),
      );
    }

    // In development, bundles can be created lazily. This means that the parent bundle may not
    // know about all of the sibling bundles of a child when it is written for the first time.
    // Therefore, we need to also ensure that the siblings are loaded when the child loads.
    if (options.shouldBuildLazily && bundle.env.outputFormat === 'global') {
      let referenced = bundleGraph.getReferencedBundles(bundle);
      for (let referencedBundle of referenced) {
        let loaders = getLoaders(bundle.env);
        if (!loaders) {
          continue;
        }

        let loader = loaders[referencedBundle.type];
        if (!loader) {
          continue;
        }

        let relativePathExpr = getRelativePathExpr(
          bundle,
          referencedBundle,
          options,
        );
        let loaderCode = `require(${JSON.stringify(
          loader,
        )})(${getAbsoluteUrlExpr(
          relativePathExpr,
          bundle,
          // $FlowFixMe
          config.domainSharding,
        )})`;
        assets.push({
          filePath: __filename,
          code: loaderCode,
          isEntry: true,
          env: {sourceType: 'module'},
        });
      }
    }

    if (
      shouldUseRuntimeManifest(bundle, options) &&
      bundleGraph
        .getChildBundles(bundle)
        .some((b) => b.bundleBehavior !== 'inline') &&
      isNewContext(bundle, bundleGraph)
    ) {
      assets.push({
        filePath: __filename,
        code: getRegisterCode(bundle, bundleGraph),
        isEntry: true,
        env: {sourceType: 'module'},
        priority: getManifestBundlePriority(
          bundleGraph,
          bundle,
          // $FlowFixMe
          config.splitManifestThreshold,
        ),
      });
    }

    return assets;
  },
}): Runtime<JSRuntimeConfig>);

function getDependencies(bundle: NamedBundle): {|
  asyncDependencies: Array<Dependency>,
  conditionalDependencies: Array<Dependency>,
  otherDependencies: Array<Dependency>,
|} {
  let cachedDependencies = bundleDependencies.get(bundle);

  if (cachedDependencies) {
    return cachedDependencies;
  } else {
    let asyncDependencies = [];
    let otherDependencies = [];
    let conditionalDependencies = [];
    bundle.traverse((node) => {
      if (node.type !== 'dependency') {
        return;
      }

      let dependency = node.value;
      if (
        dependency.priority === 'lazy' &&
        dependency.specifierType !== 'url'
      ) {
        asyncDependencies.push(dependency);
      } else if (dependency.priority === 'conditional') {
        conditionalDependencies.push(dependency);
      } else {
        otherDependencies.push(dependency);
      }
    });
    bundleDependencies.set(bundle, {
      asyncDependencies,
      conditionalDependencies,
      otherDependencies,
    });
    return {asyncDependencies, conditionalDependencies, otherDependencies};
  }
}

function getLoaderRuntime({
  bundle,
  dependency,
  bundleGroup,
  bundleGraph,
  options,
  shardingConfig,
}: {|
  bundle: NamedBundle,
  dependency: Dependency,
  bundleGroup: BundleGroup,
  bundleGraph: BundleGraph<NamedBundle>,
  options: PluginOptions,
  shardingConfig: JSRuntimeConfig['domainSharding'],
|}): ?RuntimeAsset {
  let loaders = getLoaders(bundle.env);
  if (loaders == null) {
    return;
  }

  let externalBundles = bundleGraph.getBundlesInBundleGroup(bundleGroup);
  let potentialMainBundle;

  if (getFeatureFlag('supportWebpackChunkName')) {
    potentialMainBundle = externalBundles.find((bundle) =>
      bundle
        .getEntryAssets()
        .some((asset) => asset.id === bundleGroup.entryAssetId),
    );
  } else {
    potentialMainBundle = externalBundles.find(
      (bundle) => bundle.getMainEntry()?.id === bundleGroup.entryAssetId,
    );
  }

  let mainBundle = nullthrows(potentialMainBundle);

  // CommonJS is a synchronous module system, so there is no need to load bundles in parallel.
  // Importing of the other bundles will be handled by the bundle group entry.
  // Do the same thing in library mode for ES modules, as we are building for another bundler
  // and the imports for sibling bundles will be in the target bundle.

  // Previously we also did this when building lazily, however it seemed to cause issues in some cases.
  // The original comment as to why is left here, in case a future traveller is trying to fix that issue:
  // > [...] the runtime itself could get deduplicated and only exist in the parent. This causes errors if an
  // > old version of the parent without the runtime
  // > is already loaded.
  if (bundle.env.outputFormat === 'commonjs' || bundle.env.isLibrary) {
    externalBundles = [mainBundle];
  } else {
    // Otherwise, load the bundle group entry after the others.
    externalBundles.splice(externalBundles.indexOf(mainBundle), 1);
    externalBundles.reverse().push(mainBundle);
  }

  // Determine if we need to add a dynamic import() polyfill, or if all target browsers support it natively.
  let needsDynamicImportPolyfill =
    !bundle.env.isLibrary && !bundle.env.supports('dynamic-import', true);

  let needsEsmLoadPrelude = false;
  let loaderModules: Array<string> = [];

  function getLoaderForBundle(
    bundle: NamedBundle,
    to: NamedBundle,
    shardingConfig: JSRuntimeConfig['domainSharding'],
  ): string | void {
    let loader = loaders[to.type];
    if (!loader) {
      return;
    }

    if (
      to.type === 'js' &&
      to.env.outputFormat === 'esmodule' &&
      !needsDynamicImportPolyfill &&
      shouldUseRuntimeManifest(bundle, options)
    ) {
      needsEsmLoadPrelude = true;
      return `load(${JSON.stringify(to.publicId)})`;
    }

    let relativePathExpr = getRelativePathExpr(bundle, to, options);

    // Use esmodule loader if possible
    if (to.type === 'js' && to.env.outputFormat === 'esmodule') {
      if (!needsDynamicImportPolyfill) {
        return `__parcel__import__("./" + ${relativePathExpr})`;
      }

      loader = nullthrows(
        loaders.IMPORT_POLYFILL,
        `No import() polyfill available for context '${bundle.env.context}'`,
      );
    } else if (to.type === 'js' && to.env.outputFormat === 'commonjs') {
      return `Promise.resolve(__parcel__require__("./" + ${relativePathExpr}))`;
    }

    let absoluteUrlExpr;
    if (shouldUseRuntimeManifest(bundle, options)) {
      let publicId = JSON.stringify(to.publicId);
      absoluteUrlExpr = `require('./helpers/bundle-manifest').resolve(${publicId})`;

      if (shardingConfig) {
        absoluteUrlExpr = `require('@atlaspack/domain-sharding').shardUrl(${absoluteUrlExpr}, ${shardingConfig.maxShards})`;
      }
    } else {
      absoluteUrlExpr = getAbsoluteUrlExpr(
        relativePathExpr,
        bundle,
        shardingConfig,
      );
    }

    let code = `require(${JSON.stringify(loader)})(${absoluteUrlExpr})`;

    // In development, clear the require cache when an error occurs so the
    // user can try again (e.g. after fixing a build error).
    if (
      options.mode === 'development' &&
      bundle.env.outputFormat === 'global'
    ) {
      code +=
        '.catch(err => {delete module.bundle.cache[module.id]; throw err;})';
    }
    return code;
  }

  function getConditionalLoadersForCondition(
    dependencies: Dependency[],
    sourceBundle: NamedBundle,
  ): string[] {
    if (dependencies.length === 0) {
      // Avoid extra work if there are no dependencies, so we don't have to traverse conditions
      return [];
    }

    // Get all the condition objects for the conditional dependencies
    const conditions = bundleGraph.getConditionsForDependencies(
      dependencies,
      sourceBundle,
    );

    const loaders = [];
    for (const cond of conditions) {
      // This bundle has a conditional dependency, we need to load the bundle group
      const ifTrueLoaders = cond.ifTrueBundles
        .flatMap((targetBundle) =>
          getConditionalLoadersForCondition(
            getDependencies(targetBundle).conditionalDependencies,
            targetBundle,
          ),
        )
        .concat(
          cond.ifTrueBundles.map((targetBundle) =>
            getLoaderForBundle(sourceBundle, targetBundle),
          ),
        );

      const ifFalseLoaders = cond.ifFalseBundles
        .flatMap((targetBundle) =>
          getConditionalLoadersForCondition(
            getDependencies(targetBundle).conditionalDependencies,
            targetBundle,
          ),
        )
        .concat(
          cond.ifFalseBundles.map((targetBundle) =>
            getLoaderForBundle(sourceBundle, targetBundle),
          ),
        );

      if (ifTrueLoaders.length > 0 || ifFalseLoaders.length > 0) {
        // Load conditional bundles with helper (and a dev mode with additional hints)
        loaders.push(
          `require('./helpers/conditional-loader${
            options.mode === 'development' ? '-dev' : ''
          }')('${
            cond.key
          }', function (){return Promise.all([${ifTrueLoaders.join(
            ',',
          )}]);}, function (){return Promise.all([${ifFalseLoaders.join(
            ',',
          )}]);})`,
        );
      }
    }

    return loaders;
  }

  if (getFeatureFlag('conditionalBundlingApi')) {
    let conditionalDependencies = externalBundles.flatMap(
      (to) => getDependencies(to).conditionalDependencies,
    );

    loaderModules.push(
      ...getConditionalLoadersForCondition(conditionalDependencies, bundle),
    );
  }

  for (let to of externalBundles) {
    let loaderModule = getLoaderForBundle(bundle, to, shardingConfig);
    if (loaderModule !== undefined) loaderModules.push(loaderModule);
  }

  // Similar to the comment above, this also used to be skipped when shouldBuildLazily was true,
  // however it caused issues where a bundle group contained multiple bundles.
  if (bundle.env.context === 'browser') {
    loaderModules.push(
      ...externalBundles
        // TODO: Allow css to preload resources as well
        .filter((to) => to.type === 'js')
        .flatMap((from) => {
          let {preload, prefetch} = getHintedBundleGroups(bundleGraph, from);

          return [
            ...getHintLoaders(
              bundleGraph,
              bundle,
              preload,
              BROWSER_PRELOAD_LOADER,
              options,
            ),
            ...getHintLoaders(
              bundleGraph,
              bundle,
              prefetch,
              BROWSER_PREFETCH_LOADER,
              options,
            ),
          ];
        }),
    );
  }

  if (loaderModules.length === 0) {
    return;
  }

  let loaderCode = loaderModules.join(', ');
  if (loaderModules.length > 1) {
    loaderCode = `Promise.all([${loaderCode}])`;
  } else {
    loaderCode = `(${loaderCode})`;
  }

  if (mainBundle.type === 'js') {
    let parcelRequire =
      getFeatureFlag('hmrImprovements') || bundle.env.shouldScopeHoist
        ? 'parcelRequire'
        : 'module.bundle.root';

    loaderCode += `.then(() => ${parcelRequire}('${bundleGraph.getAssetPublicId(
      bundleGraph.getAssetById(bundleGroup.entryAssetId),
    )}'))`;
  }

  if (needsEsmLoadPrelude && options.featureFlags.importRetry) {
    loaderCode = `
      Object.defineProperty(module, 'exports', { get: () => {
        let load = require('./helpers/browser/esm-js-loader-retry');
        return ${loaderCode}.then((v) => {
          Object.defineProperty(module, "exports", { value: Promise.resolve(v) })
          return v
        });
      }})`;

    return {
      filePath: __filename,
      code: loaderCode,
      dependency,
      env: {sourceType: 'module'},
    };
  }

  let code = [];

  if (needsEsmLoadPrelude) {
    let preludeLoad = shardingConfig
      ? `let load = require('./helpers/browser/esm-js-loader-shards')(${shardingConfig.maxShards});`
      : `let load = require('./helpers/browser/esm-js-loader');`;

    code.push(preludeLoad);
  }

  code.push(`module.exports = ${loaderCode};`);

  return {
    filePath: __filename,
    code: code.join('\n'),
    dependency,
    env: {sourceType: 'module'},
  };
}

function getHintedBundleGroups(
  bundleGraph: BundleGraph<NamedBundle>,
  bundle: NamedBundle,
): {|preload: Array<BundleGroup>, prefetch: Array<BundleGroup>|} {
  let preload = [];
  let prefetch = [];
  let {asyncDependencies} = getDependencies(bundle);
  for (let dependency of asyncDependencies) {
    let attributes = dependency.meta?.importAttributes;
    if (
      typeof attributes === 'object' &&
      attributes != null &&
      // $FlowFixMe
      (attributes.preload || attributes.prefetch)
    ) {
      let resolved = bundleGraph.resolveAsyncDependency(dependency, bundle);
      if (resolved?.type === 'bundle_group') {
        // === true for flow
        if (attributes.preload === true) {
          preload.push(resolved.value);
        }
        if (attributes.prefetch === true) {
          prefetch.push(resolved.value);
        }
      }
    }
  }

  return {preload, prefetch};
}

function getHintLoaders(
  bundleGraph: BundleGraph<NamedBundle>,
  from: NamedBundle,
  bundleGroups: Array<BundleGroup>,
  loader: string,
  options: PluginOptions,
): Array<string> {
  let hintLoaders = [];
  for (let bundleGroupToPreload of bundleGroups) {
    let bundlesToPreload =
      bundleGraph.getBundlesInBundleGroup(bundleGroupToPreload);

    for (let bundleToPreload of bundlesToPreload) {
      let relativePathExpr = getRelativePathExpr(
        from,
        bundleToPreload,
        options,
      );
      let priority = TYPE_TO_RESOURCE_PRIORITY[bundleToPreload.type];
      hintLoaders.push(
        `require(${JSON.stringify(loader)})(${getAbsoluteUrlExpr(
          relativePathExpr,
          from,
        )}, ${priority ? JSON.stringify(priority) : 'null'}, ${JSON.stringify(
          bundleToPreload.target.env.outputFormat === 'esmodule',
        )})`,
      );
    }
  }

  return hintLoaders;
}

function isNewContext(
  bundle: NamedBundle,
  bundleGraph: BundleGraph<NamedBundle>,
): boolean {
  let parents = bundleGraph.getParentBundles(bundle);
  let isInEntryBundleGroup = bundleGraph
    .getBundleGroupsContainingBundle(bundle)
    .some((g) => bundleGraph.isEntryBundleGroup(g));
  return (
    isInEntryBundleGroup ||
    parents.length === 0 ||
    parents.some(
      (parent) =>
        parent.env.context !== bundle.env.context || parent.type !== 'js',
    )
  );
}

function getURLRuntime(
  dependency: Dependency,
  from: NamedBundle,
  to: NamedBundle,
  options: PluginOptions,
  shardingConfig: JSRuntimeConfig['domainSharding'],
): RuntimeAsset {
  let relativePathExpr;
  if (getFeatureFlag('hmrImprovements')) {
    relativePathExpr = getRelativePathExpr(from, to, options, true);
  } else {
    relativePathExpr = getRelativePathExpr(from, to, options);
  }
  let code;

  if (dependency.meta.webworker === true && !from.env.isLibrary) {
    code = `let workerURL = require('./helpers/get-worker-url');\n`;
    if (
      from.env.outputFormat === 'esmodule' &&
      from.env.supports('import-meta-url')
    ) {
      code += `let url = new __parcel__URL__(${relativePathExpr});\n`;
      code += `module.exports = workerURL(url.toString(), url.origin, ${String(
        from.env.outputFormat === 'esmodule',
      )});`;
    } else {
      code += `let bundleURL = require('./helpers/bundle-url');\n`;
      code += `let url = bundleURL.getBundleURL('${from.publicId}') + ${relativePathExpr};`;
      if (shardingConfig) {
        code += `url = require('@atlaspack/domain-sharding').shardUrl(url, ${shardingConfig.maxShards});`;
      }
      code += `module.exports = workerURL(url, bundleURL.getOrigin(url), ${String(
        from.env.outputFormat === 'esmodule',
      )});`;
    }
  } else {
    code = `module.exports = ${getAbsoluteUrlExpr(
      relativePathExpr,
      from,
      shardingConfig,
    )};`;
  }

  return {
    filePath: __filename,
    code,
    dependency,
    env: {sourceType: 'module'},
  };
}

function getRegisterCode(
  entryBundle: NamedBundle,
  bundleGraph: BundleGraph<NamedBundle>,
): string {
  let mappings = [];
  bundleGraph.traverseBundles((bundle, _, actions) => {
    if (bundle.bundleBehavior === 'inline') {
      return;
    }

    // To make the manifest as small as possible all bundle key/values are
    // serialised into a single array e.g. ['id', 'value', 'id2', 'value2'].
    // `./helpers/bundle-manifest` accounts for this by iterating index by 2
    mappings.push(
      bundle.publicId,
      relativeBundlePath(entryBundle, nullthrows(bundle), {
        leadingDotSlash: false,
      }),
    );

    if (bundle !== entryBundle && isNewContext(bundle, bundleGraph)) {
      for (let referenced of bundleGraph.getReferencedBundles(bundle)) {
        mappings.push(
          referenced.publicId,
          relativeBundlePath(entryBundle, nullthrows(referenced), {
            leadingDotSlash: false,
          }),
        );
      }
      // New contexts have their own manifests, so there's no need to continue.
      actions.skipChildren();
    }
  }, entryBundle);

  let baseUrl =
    entryBundle.env.outputFormat === 'esmodule' &&
    entryBundle.env.supports('import-meta-url')
      ? 'new __parcel__URL__("").toString()' // <-- this isn't ideal. We should use `import.meta.url` directly but it gets replaced currently
      : `require('./helpers/bundle-url').getBundleURL('${entryBundle.publicId}')`;

  return `require('./helpers/bundle-manifest').register(${baseUrl},JSON.parse(${JSON.stringify(
    JSON.stringify(mappings),
  )}));`;
}

function getRelativePathExpr(
  from: NamedBundle,
  to: NamedBundle,
  options: PluginOptions,
  isURL = to.type !== 'js',
): string {
  let relativePath = relativeBundlePath(from, to, {leadingDotSlash: false});
  let res = JSON.stringify(relativePath);
  if (getFeatureFlag('hmrImprovements')) {
    if (isURL && options.hmrOptions) {
      res += ' + "?" + Date.now()';
    }
  } else {
    if (options.hmrOptions) {
      res += ' + "?" + Date.now()';
    }
  }

  return res;
}

function getAbsoluteUrlExpr(
  relativePathExpr: string,
  fromBundle: NamedBundle,
  shardingConfig: JSRuntimeConfig['domainSharding'],
) {
  if (
    (fromBundle.env.outputFormat === 'esmodule' &&
      fromBundle.env.supports('import-meta-url')) ||
    fromBundle.env.outputFormat === 'commonjs'
  ) {
    // This will be compiled to new URL(url, import.meta.url) or new URL(url, 'file:' + __filename).
    return `new __parcel__URL__(${relativePathExpr}).toString()`;
  }

  const regularBundleUrl = `require('./helpers/bundle-url').getBundleURL('${fromBundle.publicId}') + ${relativePathExpr}`;

  if (!shardingConfig) {
    return regularBundleUrl;
  }

  return `require('@atlaspack/domain-sharding').shardUrl(${regularBundleUrl}, ${shardingConfig.maxShards})`;
}

function shouldUseRuntimeManifest(
  bundle: NamedBundle,
  options: PluginOptions,
): boolean {
  let env = bundle.env;
  return (
    !env.isLibrary &&
    bundle.bundleBehavior !== 'inline' &&
    env.isBrowser() &&
    options.mode === 'production'
  );
}

function getManifestBundlePriority(
  bundleGraph: BundleGraph<NamedBundle>,
  bundle: NamedBundle,
  threshold: number,
): $PropertyType<RuntimeAsset, 'priority'> {
  let bundleSize = 0;

  bundle.traverseAssets((asset, _, actions) => {
    bundleSize += asset.stats.size;

    if (bundleSize > threshold) {
      actions.stop();
    }
  });

  return bundleSize > threshold ? 'parallel' : 'sync';
}
