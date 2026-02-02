/**
 * Shared utilities for BundleGraphRequest and BundleGraphRequestRust.
 *
 * This module contains common functionality used by both the JS and native
 * bundling paths, such as bundle validation, naming, and configuration loading.
 */
import type {Async, Bundle as IBundle, Namer} from '@atlaspack/types';
import type {SharedReference} from '@atlaspack/workers';
import type {AtlaspackConfig, LoadedPlugin} from '../AtlaspackConfig';
import type {RunAPI} from '../RequestTracker';
import type {
  Bundle as InternalBundle,
  Config,
  DevDepRequest,
  AtlaspackOptions,
  DevDepRequestRef,
} from '../types';

import assert from 'assert';
import nullthrows from 'nullthrows';
import {PluginLogger} from '@atlaspack/logger';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import {unique, setSymmetricDifference} from '@atlaspack/utils';
import InternalBundleGraph from '../BundleGraph';
import BundleGraph from '../public/BundleGraph';
import {Bundle, NamedBundle} from '../public/Bundle';
import PluginOptions from '../public/PluginOptions';
import {createConfig} from '../InternalConfig';
import {
  createDevDependency,
  runDevDepRequest as runDevDepRequestInternal,
} from './DevDepRequest';
import {loadPluginConfig, runConfigRequest, PluginWithLoadConfig} from './ConfigRequest';
import {joinProjectPath, fromProjectPathRelative, toProjectPathUnsafe} from '../projectPath';
import {tracer, PluginTracer} from '@atlaspack/profiler';
import type {BundleGraphResult} from './BundleGraphRequest';

/**
 * Validates that all bundles have unique names.
 * Throws an assertion error if duplicate bundle names are found.
 */
export function validateBundles(bundleGraph: InternalBundleGraph): void {
  const bundles = bundleGraph.getBundles();

  const bundleNames = bundles.map((b) =>
    joinProjectPath(b.target.distDir, nullthrows(b.name)),
  );
  assert.deepEqual(
    bundleNames,
    unique(bundleNames),
    'Bundles must have unique name. Conflicting names: ' +
      [
        ...setSymmetricDifference(
          new Set(bundleNames),
          new Set(unique(bundleNames)),
        ),
      ].join(),
  );
}

/**
 * Names a bundle by running through the configured namers until one returns a name.
 */
export async function nameBundle(
  namers: Array<LoadedPlugin<Namer<unknown>>>,
  internalBundle: InternalBundle,
  internalBundleGraph: InternalBundleGraph,
  options: AtlaspackOptions,
  pluginOptions: PluginOptions,
  configs: Map<string, Config>,
): Promise<void> {
  const bundle = Bundle.get(internalBundle, internalBundleGraph, options);
  const bundleGraph = new BundleGraph<IBundle>(
    internalBundleGraph,
    NamedBundle.get.bind(NamedBundle),
    options,
  );

  for (const namer of namers) {
    let measurement;
    try {
      measurement = tracer.createMeasurement(namer.name, 'namer', bundle.id);
      const name = await namer.plugin.name({
        bundle,
        bundleGraph,
        config: configs.get(namer.name)?.result,
        options: pluginOptions,
        logger: new PluginLogger({origin: namer.name}),
        tracer: new PluginTracer({origin: namer.name, category: 'namer'}),
      });

      if (name != null) {
        internalBundle.name = name;
        const {hashReference} = internalBundle;
        internalBundle.displayName = name.includes(hashReference)
          ? name.replace(hashReference, '[hash]')
          : name;

        return;
      }
    } catch (e: any) {
      throw new ThrowableDiagnostic({
        diagnostic: errorToDiagnostic(e, {
          origin: namer.name,
        }),
      });
    } finally {
      measurement && measurement.end();
    }
  }

  throw new Error('Unable to name bundle');
}

/**
 * Loads configuration for a plugin and tracks its dev dependencies.
 */
export async function loadPluginConfigWithDevDeps<T extends PluginWithLoadConfig>(
  plugin: LoadedPlugin<T>,
  options: AtlaspackOptions,
  api: RunAPI<BundleGraphResult>,
  previousDevDeps: Map<string, string>,
  devDepRequests: Map<string, DevDepRequest | DevDepRequestRef>,
  configs: Map<string, Config>,
): Promise<void> {
  const config = createConfig({
    plugin: plugin.name,
    searchPath: toProjectPathUnsafe('index'),
  });

  await loadPluginConfig(plugin, config, options);
  await runConfigRequest(api, config);
  for (const devDep of config.devDeps) {
    const devDepRequest = await createDevDependency(
      devDep,
      previousDevDeps,
      options,
    );
    await runDevDepRequest(api, devDepRequest, devDepRequests);
  }

  configs.set(plugin.name, config);
}

/**
 * Runs a dev dependency request and tracks it in the devDepRequests map.
 */
export async function runDevDepRequest(
  api: RunAPI<BundleGraphResult>,
  devDepRequest: DevDepRequest | DevDepRequestRef,
  devDepRequests: Map<string, DevDepRequest | DevDepRequestRef>,
): Promise<void> {
  const {specifier, resolveFrom} = devDepRequest;
  const key = `${specifier}:${fromProjectPathRelative(resolveFrom)}`;
  devDepRequests.set(key, devDepRequest);
  await runDevDepRequestInternal(api, devDepRequest);
}
