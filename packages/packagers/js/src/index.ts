import type {Async, BundleResult} from '@atlaspack/types';
import type SourceMap from '@parcel/source-map';
import {Packager} from '@atlaspack/plugin';
import {
  replaceInlineReferences,
  replaceURLReferences,
  validateSchema,
  SchemaEntity,
  debugTools,
} from '@atlaspack/utils';
import {encodeJSONKeyComponent} from '@atlaspack/diagnostic';
import {hashString} from '@atlaspack/rust';
import nullthrows from 'nullthrows';
import {DevPackager} from './DevPackager';
import {
  type PackageResult as ScopeHoistingPackageResult,
  ScopeHoistingPackager,
} from './ScopeHoistingPackager';

type JSPackagerConfig = {
  parcelRequireName: string;
  unstable_asyncBundleRuntime: boolean;
  unstable_manualStaticBindingExports: string[] | null;
};

const CONFIG_SCHEMA: SchemaEntity = {
  type: 'object',
  properties: {
    unstable_asyncBundleRuntime: {
      type: 'boolean',
    },
    unstable_manualStaticBindingExports: {
      type: 'array',
      items: {
        type: 'string',
      },
    },
  },
  additionalProperties: false,
};

export default new Packager({
  async loadConfig({config, options}): Promise<JSPackagerConfig> {
    let packageKey = '@atlaspack/packager-js';
    let conf = await config.getConfigFrom<{
      unstable_asyncBundleRuntime?: boolean;
      unstable_manualStaticBindingExports?: string[];
    }>(options.projectRoot + '/index', [], {
      packageKey,
    });

    if (conf?.contents) {
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
    }

    // Generate a name for the global parcelRequire function that is unique to this project.
    // This allows multiple parcel builds to coexist on the same page.
    let packageName = await config.getConfigFrom<string>(
      options.projectRoot + '/index',
      [],
      {
        packageKey: 'name',
      },
    );

    let name = packageName?.contents ?? '';
    return {
      parcelRequireName: 'parcelRequire' + hashString(name).slice(-4),
      unstable_asyncBundleRuntime: Boolean(
        conf?.contents?.unstable_asyncBundleRuntime,
      ),
      unstable_manualStaticBindingExports:
        conf?.contents?.unstable_manualStaticBindingExports ?? null,
    };
  },
  async package({
    bundle,
    bundleGraph,
    getInlineBundleContents,
    getSourceMapReference,
    config,
    options,
    logger,
  }) {
    // If this is a non-module script, and there is only one asset with no dependencies,
    // then we don't need to package at all and can pass through the original code un-wrapped.
    let contents, map;
    let scopeHoistingStats: ScopeHoistingPackageResult['scopeHoistingStats'];

    if (bundle.env.sourceType === 'script') {
      let entries = bundle.getEntryAssets();
      if (
        entries.length === 1 &&
        bundleGraph.getDependencies(entries[0]).length === 0
      ) {
        contents = await entries[0].getCode();
        map = await entries[0].getMap();
      }
    }

    if (contents == null) {
      if (bundle.env.shouldScopeHoist) {
        let packager = new ScopeHoistingPackager(
          options,
          bundleGraph,
          bundle,
          nullthrows(config).parcelRequireName,
          nullthrows(config).unstable_asyncBundleRuntime,
          nullthrows(config).unstable_manualStaticBindingExports,
          logger,
        );

        let packageResult = await packager.package();
        ({contents, map} = packageResult);
        scopeHoistingStats = packageResult.scopeHoistingStats;
      } else {
        let packager = new DevPackager(
          options,
          bundleGraph,
          bundle,
          nullthrows(config).parcelRequireName,
        );

        let packageResult = await packager.package();
        ({contents, map} = packageResult);
      }
    }

    contents += '\n' + (await getSourceMapSuffix(getSourceMapReference, map));

    // For library builds, we need to replace URL references with their final resolved paths.
    // For non-library builds, this is handled in the JS runtime.
    if (bundle.env.isLibrary) {
      ({contents, map} = replaceURLReferences({
        bundle,
        bundleGraph,
        contents,
        map,
        getReplacement: (s) => JSON.stringify(s).slice(1, -1),
      }));
    }

    let result = await replaceInlineReferences({
      bundle,
      bundleGraph,
      contents,
      getInlineReplacement: (dependency, inlineType, content) => ({
        from: `"${dependency.id}"`,
        to: inlineType === 'string' ? JSON.stringify(content) : content,
      }),
      getInlineBundleContents,
      map,
    });

    if (debugTools['scope-hoisting-stats']) {
      return {...result, scopeHoistingStats};
    }

    return result;
  },
}) as Packager<unknown, unknown>;

async function getSourceMapSuffix(
  getSourceMapReference: (
    arg1?: SourceMap | null | undefined,
  ) => Async<string | null | undefined>,
  map?: SourceMap | null,
): Promise<string> {
  let sourcemapReference = await getSourceMapReference(map);
  if (sourcemapReference != null) {
    return '//# sourceMappingURL=' + sourcemapReference + '\n';
  } else {
    return '';
  }
}
