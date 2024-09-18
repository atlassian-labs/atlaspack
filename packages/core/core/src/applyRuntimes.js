// @flow strict-local

import type {ContentKey} from '@atlaspack/graph';
import type {Dependency, NamedBundle as INamedBundle} from '@atlaspack/types';
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

type RuntimeConnection = {|
  bundle: InternalBundle,
  assetGroup: AssetGroup,
  dependency: ?Dependency,
  isEntry: ?boolean,
|};

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
 * Usually, the assets returned from a runtime will go in the same bundle. It is
 * possible though, for a runtime to return an asset with a `parallel` priority,
 * which allows it to be moved to a separate bundle. In practice, this is
 * usually used to generate application manifest files.
 *
 * When adding a manifest bundle (a whole new separate bundle) during a runtime,
 * it needs to be added to a bundle group which will be potentially referenced
 * by another bundle group. To avoid trying to reference a manifest entry which
 * hasn't been created yet, we process the bundles from the bottom up, so that
 * children will always be available when parents try to reference them.
 *
 * However, when merging those connections in to the bundle graph, the reversed
 * order can create a situation where the child bundles thought they were coming
 * first and so take responsibility for loading in shared bundles. When the
 * parents actually load first, they're expecting bundles to be loaded which
 * aren't yet, creating module not found errors.
 *
 * To fix that, we restore the forward topological order once all the
 * connections are created.
 *
 * Introduction of manifest bundles: https://github.com/parcel-bundler/parcel/pull/8837
 * Introduction of reverse topology: https://github.com/parcel-bundler/parcel/pull/8981
 */

export default async function applyRuntimes<TResult: RequestResult>({
  bundleGraph,
  config,
  options,
  pluginOptions,
  api,
  optionsRef,
  previousDevDeps,
  devDepRequests,
  configs,
}: {|
  bundleGraph: InternalBundleGraph,
  config: AtlaspackConfig,
  options: AtlaspackOptions,
  optionsRef: SharedReference,
  pluginOptions: PluginOptions,
  api: RunAPI<TResult>,
  previousDevDeps: Map<string, string>,
  devDepRequests: Map<string, DevDepRequest>,
  configs: Map<string, Config>,
|}): Promise<Map<string, Asset>> {
  let runtimes = await config.getRuntimes();

  // Sort bundles into reverse topological order
  let bundles = [];
  bundleGraph.traverseBundles({
    exit(bundle) {
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

              nameRuntimeBundle(connectionBundle, bundle);
            }

            connectionMap.get(connectionBundle).push({
              bundle: connectionBundle,
              assetGroup,
              dependency,
              isEntry,
            });
          }
        }
      } catch (e) {
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

  // Sort the bundles back into forward topological order
  let connections: Array<RuntimeConnection> = [];

  bundleGraph.traverseBundles({
    enter(bundle) {
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

  let {assetGraph: runtimesAssetGraph, changedAssets} =
    await reconcileNewRuntimes(api, connections, optionsRef);

  let runtimesGraph = InternalBundleGraph.fromAssetGraph(
    runtimesAssetGraph,
    options.mode === 'production',
    bundleGraph._publicIdByAssetId,
    bundleGraph._assetPublicIds,
  );

  // Merge the runtimes graph into the main bundle graph.
  bundleGraph.merge(runtimesGraph);
  for (let [assetId, publicId] of runtimesGraph._publicIdByAssetId) {
    bundleGraph._publicIdByAssetId.set(assetId, publicId);
    bundleGraph._assetPublicIds.add(publicId);
  }

  for (let {bundle, assetGroup, dependency, isEntry} of connections) {
    let assetGroupNode = nodeFromAssetGroup(assetGroup);
    let assetGroupAssetNodeIds = runtimesAssetGraph.getNodeIdsConnectedFrom(
      runtimesAssetGraph.getNodeIdByContentKey(assetGroupNode.id),
    );
    invariant(assetGroupAssetNodeIds.length === 1);
    let runtimeNodeId = assetGroupAssetNodeIds[0];
    let runtimeNode = nullthrows(runtimesAssetGraph.getNode(runtimeNodeId));
    invariant(runtimeNode.type === 'asset');

    let resolution =
      dependency &&
      bundleGraph.getResolvedAsset(
        dependencyToInternalDependency(dependency),
        bundle,
      );

    let runtimesGraphRuntimeNodeId = runtimesGraph._graph.getNodeIdByContentKey(
      runtimeNode.id,
    );
    let duplicatedContentKeys: Set<ContentKey> = new Set();
    runtimesGraph._graph.traverse((nodeId, _, actions) => {
      let node = nullthrows(runtimesGraph._graph.getNode(nodeId));
      if (node.type !== 'dependency') {
        return;
      }

      let assets = runtimesGraph._graph
        .getNodeIdsConnectedFrom(nodeId)
        .map(assetNodeId => {
          let assetNode = nullthrows(runtimesGraph._graph.getNode(assetNodeId));
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
    }, runtimesGraphRuntimeNodeId);

    let bundleNodeId = bundleGraph._graph.getNodeIdByContentKey(bundle.id);
    let bundleGraphRuntimeNodeId = bundleGraph._graph.getNodeIdByContentKey(
      runtimeNode.id,
    ); // the node id is not constant between graphs

    runtimesGraph._graph.traverse((nodeId, _, actions) => {
      let node = nullthrows(runtimesGraph._graph.getNode(nodeId));
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
    }, runtimesGraphRuntimeNodeId);

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

function reconcileNewRuntimes<TResult: RequestResult>(
  api: RunAPI<TResult>,
  connections: Array<RuntimeConnection>,
  optionsRef: SharedReference,
) {
  let assetGroups = connections.map(t => t.assetGroup);
  let request = createAssetGraphRequest({
    name: 'Runtimes',
    assetGroups,
    optionsRef,
  });

  // rebuild the graph
  return api.runRequest(request, {force: true});
}
