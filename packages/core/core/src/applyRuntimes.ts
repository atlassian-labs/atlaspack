import type {ContentKey} from '@atlaspack/graph';
import type {Dependency, NamedBundle as INamedBundle} from '@atlaspack/types';
// @ts-expect-error - TS2614 - Module '"@atlaspack/workers"' has no exported member 'SharedReference'. Did you mean to use 'import SharedReference from "@atlaspack/workers"' instead?
import type {SharedReference} from '@atlaspack/workers';
import type {
  Asset,
  AssetGroup,
  Bundle as InternalBundle,
  Config,
  DevDepRequest,
  AtlaspackOptions,
} from './types';
import type AtlaspackConfig from './AtlaspackConfig';
import type PluginOptions from './public/PluginOptions';
import type {RequestResult, RunAPI} from './RequestTracker';

import path from 'path';
import assert from 'assert';
import invariant from 'assert';
import nullthrows from 'nullthrows';
import {nodeFromAssetGroup} from './AssetGraph';
import BundleGraph from './public/BundleGraph';
import InternalBundleGraph, {bundleGraphEdgeTypes} from './BundleGraph';
import {NamedBundle} from './public/Bundle';
import {PluginLogger} from '@atlaspack/logger';
import {hashString} from '@atlaspack/rust';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import {dependencyToInternalDependency} from './public/Dependency';
import {mergeEnvironments} from './Environment';
import createAssetGraphRequest from './requests/AssetGraphRequest';
import {createDevDependency, runDevDepRequest} from './requests/DevDepRequest';
import {toProjectPath, fromProjectPathRelative} from './projectPath';
import {tracer, PluginTracer} from '@atlaspack/profiler';
import {DefaultMap} from '@atlaspack/utils';

type RuntimeConnection = {
  bundle: InternalBundle;
  assetGroup: AssetGroup;
  dependency: Dependency | null | undefined;
  isEntry: boolean | null | undefined;
};

function nameRuntimeBundle(
  bundle: InternalBundle,
  siblingBundle: InternalBundle,
) {
  // We don't run custom namers on runtime bundles as the runtime assumes that they are
  // located at the same nesting level as their owning bundle. Custom naming could
  // be added in future as long as the custom name is validated.
  let {hashReference} = bundle;

  let name = nullthrows(siblingBundle.name)
    // Remove the existing hash from standard file patterns
    // e.g. 'main.[hash].js' -> 'main.js' or 'main~[hash].js' -> 'main.js'
    .replace(new RegExp(`[\\.~\\-_]?${siblingBundle.hashReference}`), '')
    // Ensure the file ends with 'runtime.[hash].js'
    .replace(`.${bundle.type}`, `.runtime.${hashReference}.${bundle.type}`);

  bundle.name = name;
  bundle.displayName = name.replace(hashReference, '[hash]');
}

/**
 * The applyRuntimes function is responsible for generating all the runtimes
 * (assets created during the build that don't actually exist on disk) and then
 * linking them into the bundle graph.
 *
 * Introduction of manifest bundles: https://github.com/parcel-bundler/parcel/pull/8837
 * Introduction of reverse topology: https://github.com/parcel-bundler/parcel/pull/8981
 */
export default async function applyRuntimes<TResult extends RequestResult>({
  bundleGraph,
  config,
  options,
  pluginOptions,
  api,
  optionsRef,
  previousDevDeps,
  devDepRequests,
  configs,
}: {
  bundleGraph: InternalBundleGraph;
  config: AtlaspackConfig;
  options: AtlaspackOptions;
  optionsRef: SharedReference;
  pluginOptions: PluginOptions;
  api: RunAPI<TResult>;
  previousDevDeps: Map<string, string>;
  devDepRequests: Map<string, DevDepRequest>;
  configs: Map<string, Config>;
}): Promise<Map<string, Asset>> {
  let runtimes = await config.getRuntimes();

  /**
   * Usually, the assets returned from a runtime will go in the same bundle. It is
   * possible though, for a runtime to return an asset with a `parallel` priority,
   * which allows it to be moved to a separate bundle. In practice, this is
   * usually used to generate application manifest files.
   *
   * When adding a manifest bundle (a whole new separate bundle) during a runtime,
   * it needs to be added to a bundle group which will be potentially referenced
   * by another bundle group. To avoid trying to reference a manifest entry which
   * hasn't been created yet, we process the bundles from the bottom up (topological
   * order), so that children will always be available when parents try to reference
   * them.
   */
  let bundles: Array<Bundle> = [];
  bundleGraph.traverseBundles({
    exit(bundle: Bundle) {
      bundles.push(bundle);
    },
  });

  let connectionMap = new DefaultMap(() => []);

  for (let bundle of bundles) {
    for (let runtime of runtimes) {
      let measurement;
      try {
        const namedBundle = NamedBundle.get(bundle, bundleGraph, options);
        measurement = tracer.createMeasurement(
          runtime.name,
          'applyRuntime',
          namedBundle.displayName,
        );
        let applied = await runtime.plugin.apply({
          bundle: namedBundle,
          bundleGraph: new BundleGraph<INamedBundle>(
            bundleGraph,
            NamedBundle.get.bind(NamedBundle),
            options,
          ),
          config: configs.get(runtime.name)?.result,
          options: pluginOptions,
          logger: new PluginLogger({origin: runtime.name}),
          tracer: new PluginTracer({
            origin: runtime.name,
            category: 'applyRuntime',
          }),
        });

        if (applied) {
          let runtimeAssets = Array.isArray(applied) ? applied : [applied];
          for (let {
            code,
            dependency,
            filePath,
            isEntry,
            env,
            priority,
          } of runtimeAssets) {
            let sourceName = path.join(
              path.dirname(filePath),
              `runtime-${hashString(code)}.${bundle.type}`,
            );

            let assetGroup = {
              code,
              filePath: toProjectPath(options.projectRoot, sourceName),
              env: mergeEnvironments(options.projectRoot, bundle.env, env),
              // Runtime assets should be considered source, as they should be
              // e.g. compiled to run in the target environment
              isSource: true,
            };

            let connectionBundle = bundle;

            /**
             * If a runtime asset is marked with a priority of `parallel` this
             * means we need to create a new bundle for the asset and add it to
             * all the same bundle groups.
             */
            if (priority === 'parallel' && !bundle.needsStableName) {
              let bundleGroups =
                bundleGraph.getBundleGroupsContainingBundle(bundle);

              connectionBundle = nullthrows(
                bundleGraph.createBundle({
                  type: bundle.type,
                  needsStableName: false,
                  env: bundle.env,
                  target: bundle.target,
                  uniqueKey: 'runtime-manifest:' + bundle.id,
                  shouldContentHash: options.shouldContentHash,
                }),
              );

              for (let bundleGroup of bundleGroups) {
                bundleGraph.addBundleToBundleGroup(
                  connectionBundle,
                  bundleGroup,
                );
              }
              bundleGraph.createBundleReference(bundle, connectionBundle);

              // Ensure we name the bundle now as all other bundles have already
              // been named as this point.
              nameRuntimeBundle(connectionBundle, bundle);
            }

            connectionMap.get(connectionBundle).push({
              // @ts-expect-error - TS2322 - Type 'Bundle' is not assignable to type 'never'.
              bundle: connectionBundle,
              // @ts-expect-error - TS2322 - Type '{ code: string; filePath: string; env: Environment; isSource: boolean; }' is not assignable to type 'never'.
              assetGroup,
              // @ts-expect-error - TS2322 - Type 'Dependency | undefined' is not assignable to type 'never'.
              dependency,
              // @ts-expect-error - TS2322 - Type 'boolean | undefined' is not assignable to type 'never'.
              isEntry,
            });
          }
        }
      } catch (e: any) {
        throw new ThrowableDiagnostic({
          diagnostic: errorToDiagnostic(e, {
            origin: runtime.name,
          }),
        });
      } finally {
        measurement && measurement.end();
      }
    }
  }

  /**
   * When merging the connections in to the bundle graph, the topological
   * order can create module not found errors in some situations, often when HMR
   * is enabled. To fix this, we put the connections into DFS order.
   *
   * Note: While DFS order seems to be the most reliable order to process the
   * connections, this is likely due to it being close to the order that the bundles were
   * inserted into the graph. There is a known issue where runtime assets marked
   * as `isEntry` can create scenarios where there is no correct load order that
   * won't error, as the entry runtime assets are added to many bundles in a
   * single bundle group but their dependencies are not.
   *
   * This issue is almost exclusive to HMR scenarios as the two HMR runtime
   * plugins (@atlaspack/runtime-browser-hmr and @atlaspack/runtime-react-refresh)
   * are the only known cases where a runtime asset is marked as `isEntry`.
   */
  let connections: Array<RuntimeConnection> = [];
  bundleGraph.traverseBundles({
    enter(bundle: Bundle) {
      connections.push(...connectionMap.get(bundle));
    },
  });

  // Add dev deps for runtime plugins AFTER running them, to account for lazy require().
  for (let runtime of runtimes) {
    let devDepRequest = await createDevDependency(
      {
        specifier: runtime.name,
        resolveFrom: runtime.resolveFrom,
      },
      previousDevDeps,
      options,
    );
    devDepRequests.set(
      `${devDepRequest.specifier}:${fromProjectPathRelative(
        devDepRequest.resolveFrom,
      )}`,
      devDepRequest,
    );
    await runDevDepRequest(api, devDepRequest);
  }

  // Create a new AssetGraph from the generated runtime assets which also runs
  // transforms and resolves all dependencies.
  let {assetGraph: runtimesAssetGraph, changedAssets} =
    await reconcileNewRuntimes(api, connections, optionsRef);

  // Convert the runtime AssetGraph into a BundleGraph, this includes assigning
  // the assets their public ids
  let runtimesBundleGraph = InternalBundleGraph.fromAssetGraph(
    runtimesAssetGraph,
    options.mode === 'production',
    bundleGraph._publicIdByAssetId,
    bundleGraph._assetPublicIds,
  );

  // Merge the runtimes bundle graph into the main bundle graph.
  bundleGraph.merge(runtimesBundleGraph);

  // Add the public id mappings from the runtumes bundlegraph to the main bundle graph
  for (let [assetId, publicId] of runtimesBundleGraph._publicIdByAssetId) {
    bundleGraph._publicIdByAssetId.set(assetId, publicId);
    bundleGraph._assetPublicIds.add(publicId);
  }

  // Connect each of the generated runtime assets to bundles in the main bundle
  // graph. This is like a mini-bundling algorithm for runtime assets.
  for (let {bundle, assetGroup, dependency, isEntry} of connections) {
    let assetGroupNode = nodeFromAssetGroup(assetGroup);
    let assetGroupAssetNodeIds = runtimesAssetGraph.getNodeIdsConnectedFrom(
      runtimesAssetGraph.getNodeIdByContentKey(assetGroupNode.id),
    );
    invariant(assetGroupAssetNodeIds.length === 1);
    let runtimeNodeId = assetGroupAssetNodeIds[0];
    let runtimeNode = nullthrows(runtimesAssetGraph.getNode(runtimeNodeId));
    invariant(runtimeNode.type === 'asset');

    // Find the asset that the runtime asset should be connected from by resolving
    // it's dependency.
    let resolution =
      dependency &&
      bundleGraph.getResolvedAsset(
        dependencyToInternalDependency(dependency),
        bundle,
      );

    // Walk all the dependencies of the runtime assets and check if they are
    // already reachable from the bundle that the runtime asset is assigned to.
    // If so, we add them to `duplicatedContentKeys` to be skipped when assigning
    // assets to bundles.
    let runtimesBundleGraphRuntimeNodeId =
      runtimesBundleGraph._graph.getNodeIdByContentKey(runtimeNode.id);
    let duplicatedContentKeys: Set<ContentKey> = new Set();
    runtimesBundleGraph._graph.traverse((nodeId, _, actions) => {
      let node = nullthrows(runtimesBundleGraph._graph.getNode(nodeId));
      if (node.type !== 'dependency') {
        return;
      }

      let assets = runtimesBundleGraph._graph
        .getNodeIdsConnectedFrom(nodeId)
        .map((assetNodeId) => {
          let assetNode = nullthrows(
            runtimesBundleGraph._graph.getNode(assetNodeId),
          );
          invariant(assetNode.type === 'asset');
          return assetNode.value;
        });

      for (let asset of assets) {
        if (
          bundleGraph.isAssetReachableFromBundle(asset, bundle) ||
          resolution?.id === asset.id
        ) {
          duplicatedContentKeys.add(asset.id);
          actions.skipChildren();
        }
      }
    }, runtimesBundleGraphRuntimeNodeId);

    let bundleNodeId = bundleGraph._graph.getNodeIdByContentKey(bundle.id);
    let bundleGraphRuntimeNodeId = bundleGraph._graph.getNodeIdByContentKey(
      runtimeNode.id,
    ); // the node id is not constant between graphs

    // Assign the runtime assets and all of it's depepdencies to the bundle unless
    // we have detected it as already being reachable from this bundle via `duplicatedContentKeys`.
    runtimesBundleGraph._graph.traverse((nodeId, _, actions) => {
      let node = nullthrows(runtimesBundleGraph._graph.getNode(nodeId));
      if (node.type === 'asset' || node.type === 'dependency') {
        if (duplicatedContentKeys.has(node.id)) {
          actions.skipChildren();
          return;
        }

        const bundleGraphNodeId = bundleGraph._graph.getNodeIdByContentKey(
          node.id,
        ); // the node id is not constant between graphs
        bundleGraph._graph.addEdge(
          bundleNodeId,
          bundleGraphNodeId,
          bundleGraphEdgeTypes.contains,
        );
      }
    }, runtimesBundleGraphRuntimeNodeId);

    if (isEntry) {
      bundleGraph._graph.addEdge(bundleNodeId, bundleGraphRuntimeNodeId);
      bundle.entryAssetIds.unshift(runtimeNode.id);
    }

    if (dependency == null) {
      // Verify this asset won't become an island
      assert(
        bundleGraph._graph.getNodeIdsConnectedTo(bundleGraphRuntimeNodeId)
          .length > 0,
        'Runtime must have an inbound dependency or be an entry',
      );
    } else {
      let dependencyNodeId = bundleGraph._graph.getNodeIdByContentKey(
        dependency.id,
      );
      bundleGraph._graph.addEdge(dependencyNodeId, bundleGraphRuntimeNodeId);
    }
  }

  return changedAssets;
}

function reconcileNewRuntimes<TResult extends RequestResult>(
  api: RunAPI<TResult>,
  connections: Array<RuntimeConnection>,
  optionsRef: SharedReference,
) {
  let assetGroups = connections.map((t) => t.assetGroup);
  let request = createAssetGraphRequest({
    name: 'Runtimes',
    assetGroups,
    optionsRef,
  });

  // rebuild the graph
  return api.runRequest(request, {force: true});
}
