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
import fs from 'fs';
import nullthrows from 'nullthrows';
import path from 'path';
import {PluginLogger} from '@atlaspack/logger';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import {unique, setSymmetricDifference} from '@atlaspack/utils';
import InternalBundleGraph, {bundleGraphEdgeTypes} from '../BundleGraph';
import BundleGraph from '../public/BundleGraph';
import {Bundle, NamedBundle} from '../public/Bundle';
import PluginOptions from '../public/PluginOptions';
import {createConfig} from '../InternalConfig';
import {
  createDevDependency,
  runDevDepRequest as runDevDepRequestInternal,
} from './DevDepRequest';
import {
  loadPluginConfig,
  runConfigRequest,
  PluginWithLoadConfig,
} from './ConfigRequest';
import {
  joinProjectPath,
  fromProjectPathRelative,
  toProjectPathUnsafe,
} from '../projectPath';
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
 * Dump a canonical JSON snapshot of the bundle graph for parity comparison.
 * Gated by ATLASPACK_DUMP_BUNDLE_GRAPH environment variable which specifies the output directory.
 * The snapshot captures bundle identity, type, contained assets, and bundle group structure
 * in a deterministic, sorted format suitable for diffing.
 */
export function dumpBundleGraphSnapshot(
  bundleGraph: InternalBundleGraph,
  variant: 'js' | 'rust',
): void {
  let outDir = process.env.ATLASPACK_DUMP_BUNDLE_GRAPH;
  if (!outDir) return;

  let filename =
    variant === 'js' ? 'bundle-graph-js.json' : 'bundle-graph-rust.json';
  let outPath = path.join(outDir, filename);

  fs.mkdirSync(outDir, {recursive: true});

  let bundles = bundleGraph.getBundles();
  let bundlesSnapshot = bundles
    .map((bundle) => {
      let bundleNodeId = bundleGraph._graph.getNodeIdByContentKey(bundle.id);
      let containedAssetNodeIds = bundleGraph._graph.getNodeIdsConnectedFrom(
        bundleNodeId,
        bundleGraphEdgeTypes.contains,
      );
      let containedAssets = containedAssetNodeIds
        .map((nodeId) => bundleGraph._graph.getNode(nodeId))
        .flatMap((node) => {
          if (node?.type !== 'asset') return [];
          return [
            {
              id: node.value.id,
              filePath: fromProjectPathRelative(node.value.filePath),
            },
          ];
        })
        .sort((a, b) => a.filePath.localeCompare(b.filePath));

      // Resolve mainEntry and entry asset file paths
      let mainEntryPath: string | null = null;
      let entryAssetPaths: string[] = [];
      if (bundle.mainEntryId) {
        let mainEntryNodeId = bundleGraph._graph.getNodeIdByContentKey(
          bundle.mainEntryId,
        );
        let mainEntryNode = bundleGraph._graph.getNode(mainEntryNodeId);
        if (mainEntryNode?.type === 'asset') {
          mainEntryPath = fromProjectPathRelative(mainEntryNode.value.filePath);
        }
      }
      for (let entryId of bundle.entryAssetIds) {
        let entryNodeId = bundleGraph._graph.getNodeIdByContentKey(entryId);
        let entryNode = bundleGraph._graph.getNode(entryNodeId);
        if (entryNode?.type === 'asset') {
          entryAssetPaths.push(
            fromProjectPathRelative(entryNode.value.filePath),
          );
        }
      }
      entryAssetPaths.sort();

      return {
        id: bundle.id,
        type: bundle.type,
        bundleBehavior: bundle.bundleBehavior ?? null,
        needsStableName: bundle.needsStableName,
        isSplittable: bundle.isSplittable,
        isPlaceholder: bundle.isPlaceholder,
        mainEntryPath,
        entryAssetPaths,
        assets: containedAssets.map((a) => a.filePath),
      };
    })
    .sort((a, b) => {
      // Sort by mainEntryPath first, then by sorted assets as tiebreaker
      let aKey = a.mainEntryPath || a.assets.join(',');
      let bKey = b.mainEntryPath || b.assets.join(',');
      return aKey.localeCompare(bKey);
    });

  let bundleGroupsSnapshot = bundleGraph._graph.nodes
    .flatMap((node) => {
      if (node?.type !== 'bundle_group') return [];

      let bundleGroup = node.value;

      // Resolve entry asset file path
      let entryAssetPath: string | null = null;
      try {
        let entryNodeId = bundleGraph._graph.getNodeIdByContentKey(
          bundleGroup.entryAssetId,
        );
        let entryNode = bundleGraph._graph.getNode(entryNodeId);
        if (entryNode?.type === 'asset') {
          entryAssetPath = fromProjectPathRelative(entryNode.value.filePath);
        }
      } catch {
        // Content key not found
      }

      let bundlesInGroup = bundleGraph.getBundlesInBundleGroup(bundleGroup);
      let bundlePaths = bundlesInGroup
        .map((b) => {
          // Use mainEntry file path if available, otherwise bundle id as fallback
          if (b.mainEntryId) {
            try {
              let nodeId = bundleGraph._graph.getNodeIdByContentKey(
                b.mainEntryId,
              );
              let node = bundleGraph._graph.getNode(nodeId);
              if (node?.type === 'asset') {
                return fromProjectPathRelative(node.value.filePath);
              }
            } catch {
              // fallback
            }
          }
          return `[bundle:${b.id}]`;
        })
        .sort();

      return [
        {
          entryAssetPath:
            entryAssetPath ?? `[unknown:${bundleGroup.entryAssetId}]`,
          bundlePaths,
        },
      ];
    })
    .sort((a, b) => a.entryAssetPath.localeCompare(b.entryAssetPath));

  let totalAssets = bundleGraph._graph.nodes.filter(
    (node) => node?.type === 'asset',
  ).length;

  let snapshot = {
    version: 1,
    variant,
    stats: {
      totalBundles: bundlesSnapshot.length,
      totalBundleGroups: bundleGroupsSnapshot.length,
      totalAssets,
    },
    bundles: bundlesSnapshot,
    bundleGroups: bundleGroupsSnapshot,
  };

  fs.writeFileSync(outPath, JSON.stringify(snapshot, null, 2), 'utf8');
  // eslint-disable-next-line no-console
  console.log(`[BundleGraphSnapshot] Wrote ${variant} snapshot to ${outPath}`);
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
export async function loadPluginConfigWithDevDeps<
  T extends PluginWithLoadConfig,
>(
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
