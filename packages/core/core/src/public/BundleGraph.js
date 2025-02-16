// @flow strict-local

import type {
  Asset as IAsset,
  Bundle as IBundle,
  BundleGraph as IBundleGraph,
  BundleGraphTraversable,
  BundleGroup as IBundleGroup,
  Dependency as IDependency,
  ExportSymbolResolution,
  FilePath,
  GraphVisitor,
  Symbol,
  NamedBundle,
  SymbolResolution,
  Target,
} from '@atlaspack/types';
import type {Bundle as InternalBundle, AtlaspackOptions} from '../types';
import type InternalBundleGraph from '../BundleGraph';

import invariant from 'assert';
import nullthrows from 'nullthrows';

import {mapVisitor} from '@atlaspack/graph';
import {assetFromValue, assetToAssetValue, Asset} from './Asset';
import {bundleToInternalBundle} from './Bundle';
import Dependency, {
  dependencyToInternalDependency,
  getPublicDependency,
} from './Dependency';
import {targetToInternalTarget} from './Target';
import {fromInternalSourceLocation} from '../utils';
import BundleGroup, {bundleGroupToInternalBundleGroup} from './BundleGroup';

// Friendly access for other modules within this package that need access
// to the internal bundle.
const _bundleGraphToInternalBundleGraph: WeakMap<
  IBundleGraph<IBundle>,
  InternalBundleGraph,
> = new WeakMap();

export function bundleGraphToInternalBundleGraph(
  bundleGraph: IBundleGraph<IBundle>,
): InternalBundleGraph {
  return nullthrows(_bundleGraphToInternalBundleGraph.get(bundleGraph));
}

type BundleFactory<TBundle: IBundle> = (
  InternalBundle,
  InternalBundleGraph,
  AtlaspackOptions,
) => TBundle;

export default class BundleGraph<TBundle: IBundle>
  implements IBundleGraph<TBundle>
{
  #graph: InternalBundleGraph;
  #options: AtlaspackOptions;
  #createBundle: BundleFactory<TBundle>;

  constructor(
    graph: InternalBundleGraph,
    createBundle: BundleFactory<TBundle>,
    options: AtlaspackOptions,
  ) {
    this.#graph = graph;
    this.#options = options;
    this.#createBundle = createBundle;
    // $FlowFixMe
    _bundleGraphToInternalBundleGraph.set(this, graph);
  }

  getAssetById(id: string): Asset {
    return assetFromValue(this.#graph.getAssetById(id), this.#options);
  }

  getAssetPublicId(asset: IAsset): string {
    return this.#graph.getAssetPublicId(assetToAssetValue(asset));
  }

  isDependencySkipped(dep: IDependency): boolean {
    return this.#graph.isDependencySkipped(dependencyToInternalDependency(dep));
  }

  getResolvedAsset(dep: IDependency, bundle: ?IBundle): ?IAsset {
    let resolution = this.#graph.getResolvedAsset(
      dependencyToInternalDependency(dep),
      bundle && bundleToInternalBundle(bundle),
    );
    if (resolution) {
      return assetFromValue(resolution, this.#options);
    }
  }

  getIncomingDependencies(asset: IAsset): Array<IDependency> {
    return this.#graph
      .getIncomingDependencies(assetToAssetValue(asset))
      .map((dep) => getPublicDependency(dep, this.#options));
  }

  getAssetWithDependency(dep: IDependency): ?IAsset {
    let asset = this.#graph.getAssetWithDependency(
      dependencyToInternalDependency(dep),
    );
    if (asset) {
      return assetFromValue(asset, this.#options);
    }
  }

  getAllBundleGroups(): BundleGroup[] {
    const bundleGroups: BundleGroup[] = [];

    this.#graph._graph.nodes.forEach((node /*, nodeId*/) => {
      if (node?.type === 'bundle_group') {
        // const parentNodes = this.#graph._graph.getNodeIdsConnectedTo(nodeId);
        bundleGroups.push(new BundleGroup(node.value, this.#options));
      }
    });

    return bundleGroups;
  }

  getBundleGroupsContainingBundle(bundle: IBundle): Array<IBundleGroup> {
    return this.#graph
      .getBundleGroupsContainingBundle(bundleToInternalBundle(bundle))
      .map((bundleGroup) => new BundleGroup(bundleGroup, this.#options));
  }

  getReferencedBundles(
    bundle: IBundle,
    opts?: {|recursive?: boolean, includeInline?: boolean|},
  ): Array<TBundle> {
    return this.#graph
      .getReferencedBundles(bundleToInternalBundle(bundle), opts)
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  getReferencingBundles(bundle: IBundle): Array<TBundle> {
    return this.#graph
      .getReferencingBundles(bundleToInternalBundle(bundle))
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  resolveAsyncDependency(
    dependency: IDependency,
    bundle: ?IBundle,
  ): ?(
    | {|type: 'bundle_group', value: IBundleGroup|}
    | {|type: 'asset', value: IAsset|}
  ) {
    let resolved = this.#graph.resolveAsyncDependency(
      dependencyToInternalDependency(dependency),
      bundle && bundleToInternalBundle(bundle),
    );

    if (resolved == null) {
      return;
    } else if (resolved.type === 'bundle_group') {
      return {
        type: 'bundle_group',
        value: new BundleGroup(resolved.value, this.#options),
      };
    }

    return {
      type: 'asset',
      value: assetFromValue(resolved.value, this.#options),
    };
  }

  getReferencedBundle(dependency: IDependency, bundle: IBundle): ?TBundle {
    let result = this.#graph.getReferencedBundle(
      dependencyToInternalDependency(dependency),
      bundleToInternalBundle(bundle),
    );

    if (result != null) {
      return this.#createBundle(result, this.#graph, this.#options);
    }
  }

  getDependencies(asset: IAsset): Array<IDependency> {
    return this.#graph
      .getDependencies(assetToAssetValue(asset))
      .map((dep) => getPublicDependency(dep, this.#options));
  }

  isAssetReachableFromBundle(asset: IAsset, bundle: IBundle): boolean {
    return this.#graph.isAssetReachableFromBundle(
      assetToAssetValue(asset),
      bundleToInternalBundle(bundle),
    );
  }

  isAssetReferenced(bundle: IBundle, asset: IAsset): boolean {
    return this.#graph.isAssetReferenced(
      bundleToInternalBundle(bundle),
      assetToAssetValue(asset),
    );
  }

  hasParentBundleOfType(bundle: IBundle, type: string): boolean {
    return this.#graph.hasParentBundleOfType(
      bundleToInternalBundle(bundle),
      type,
    );
  }

  getBundlesInBundleGroup(
    bundleGroup: IBundleGroup,
    opts?: {|includeInline: boolean|},
  ): Array<TBundle> {
    return this.#graph
      .getBundlesInBundleGroup(
        bundleGroupToInternalBundleGroup(bundleGroup),
        opts,
      )
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  getBundles(opts?: {|includeInline: boolean|}): Array<TBundle> {
    return this.#graph
      .getBundles(opts)
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  isEntryBundleGroup(bundleGroup: IBundleGroup): boolean {
    return this.#graph.isEntryBundleGroup(
      bundleGroupToInternalBundleGroup(bundleGroup),
    );
  }

  getChildBundles(bundle: IBundle): Array<TBundle> {
    return this.#graph
      .getChildBundles(bundleToInternalBundle(bundle))
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  getParentBundles(bundle: IBundle): Array<TBundle> {
    return this.#graph
      .getParentBundles(bundleToInternalBundle(bundle))
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  getSymbolResolution(
    asset: IAsset,
    symbol: Symbol,
    boundary: ?IBundle,
  ): SymbolResolution {
    let res = this.#graph.getSymbolResolution(
      assetToAssetValue(asset),
      symbol,
      boundary ? bundleToInternalBundle(boundary) : null,
    );
    return {
      asset: assetFromValue(res.asset, this.#options),
      exportSymbol: res.exportSymbol,
      symbol: res.symbol,
      loc: fromInternalSourceLocation(this.#options.projectRoot, res.loc),
    };
  }

  getExportedSymbols(
    asset: IAsset,
    boundary: ?IBundle,
  ): Array<ExportSymbolResolution> {
    let res = this.#graph.getExportedSymbols(
      assetToAssetValue(asset),
      boundary ? bundleToInternalBundle(boundary) : null,
    );
    return res.map((e) => ({
      asset: assetFromValue(e.asset, this.#options),
      exportSymbol: e.exportSymbol,
      symbol: e.symbol,
      loc: fromInternalSourceLocation(this.#options.projectRoot, e.loc),
      exportAs: e.exportAs,
    }));
  }

  traverse<TContext>(
    visit: GraphVisitor<BundleGraphTraversable, TContext>,
    start?: ?IAsset,
    opts?: ?{|skipUnusedDependencies?: boolean|},
  ): ?TContext {
    return this.#graph.traverse(
      mapVisitor((node, actions) => {
        // Skipping unused dependencies here is faster than doing an isDependencySkipped check inside the visitor
        // because the node needs to be re-looked up by id from the hashmap.
        if (
          opts?.skipUnusedDependencies &&
          node.type === 'dependency' &&
          (node.hasDeferred || node.excluded)
        ) {
          actions.skipChildren();
          return null;
        }
        return node.type === 'asset'
          ? {type: 'asset', value: assetFromValue(node.value, this.#options)}
          : {
              type: 'dependency',
              value: getPublicDependency(node.value, this.#options),
            };
      }, visit),
      start ? assetToAssetValue(start) : undefined,
    );
  }

  traverseBundles<TContext>(
    visit: GraphVisitor<TBundle, TContext>,
    startBundle: ?IBundle,
  ): ?TContext {
    return this.#graph.traverseBundles(
      mapVisitor(
        (bundle) => this.#createBundle(bundle, this.#graph, this.#options),
        visit,
      ),
      startBundle == null ? undefined : bundleToInternalBundle(startBundle),
    );
  }

  getBundlesWithAsset(asset: IAsset): Array<TBundle> {
    return this.#graph
      .getBundlesWithAsset(assetToAssetValue(asset))
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  getBundlesWithDependency(dependency: IDependency): Array<TBundle> {
    return this.#graph
      .getBundlesWithDependency(dependencyToInternalDependency(dependency))
      .map((bundle) => this.#createBundle(bundle, this.#graph, this.#options));
  }

  getUsedSymbols(v: IAsset | IDependency): ?$ReadOnlySet<Symbol> {
    if (v instanceof Asset) {
      return this.#graph.getUsedSymbolsAsset(assetToAssetValue(v));
    } else {
      invariant(v instanceof Dependency);
      return this.#graph.getUsedSymbolsDependency(
        dependencyToInternalDependency(v),
      );
    }
  }

  getEntryRoot(target: Target): FilePath {
    return this.#graph.getEntryRoot(
      this.#options.projectRoot,
      targetToInternalTarget(target),
    );
  }

  // Given a set of dependencies, return any conditions where those dependencies are either
  // the true or false dependency for those conditions. This is currently used to work out which
  // conditions belong to a bundle in packaging.
  getConditionsForDependencies(
    deps: Array<IDependency>,
    bundle: NamedBundle,
  ): Set<{|
    publicId: string,
    key: string,
    ifTrueDependency: IDependency,
    ifFalseDependency: IDependency,
    ifTrueBundles: Array<TBundle>,
    ifFalseBundles: Array<TBundle>,
    ifTrueAssetId: string,
    ifFalseAssetId: string,
  |}> {
    const conditions = new Set();
    const depIds = deps.map((dep) => dep.id);
    for (const condition of this.#graph._conditions.values()) {
      if (
        depIds.includes(condition.ifTrueDependency.id) ||
        depIds.includes(condition.ifFalseDependency.id)
      ) {
        const [[trueAsset, ifTrueBundles], [falseAsset, ifFalseBundles]] = [
          condition.ifTrueDependency,
          condition.ifFalseDependency,
        ].map((dep) => {
          const asset = this.#graph.getResolvedAsset(
            dep,
            bundleToInternalBundle(bundle),
          );
          if (
            asset &&
            this.#graph.bundleHasAsset(bundleToInternalBundle(bundle), asset)
          ) {
            // Asset is in the same bundle, we know it doesn't need to be loaded externally
            return [asset, []];
          }

          const resolvedAsync = this.#graph.resolveAsyncDependency(
            dep,
            bundleToInternalBundle(bundle),
          );
          if (resolvedAsync?.type === 'asset') {
            // Single bundle to load dynamically
            return [
              resolvedAsync.value,
              [
                this.#createBundle(
                  nullthrows(
                    this.#graph.getReferencedBundle(
                      dep,
                      bundleToInternalBundle(bundle),
                    ),
                    `Failed to load referenced bundle for '${
                      dep.specifier
                    }' specifier from '${String(dep.sourcePath)}'`,
                  ),
                  this.#graph,
                  this.#options,
                ),
              ],
            ];
          } else if (resolvedAsync) {
            // Bundle group means we have multiple bundles to load first
            return [
              this.#graph.getAssetById(resolvedAsync.value.entryAssetId),
              this.#graph
                .getBundlesInBundleGroup(resolvedAsync.value)
                .map((b) => this.#createBundle(b, this.#graph, this.#options)),
            ];
          } else {
            return [
              nullthrows(
                asset,
                `Failed to load depenendency for '${
                  dep.specifier
                }' specifier from '${String(dep.sourcePath)}'`,
              ),
              [],
            ];
          }
        });

        conditions.add({
          publicId: condition.publicId,
          key: condition.key,
          ifTrueDependency: nullthrows(
            deps.find((dep) => dep.id === condition.ifTrueDependency.id),
            'ifTrueDependency was null',
          ),
          ifFalseDependency: nullthrows(
            deps.find((dep) => dep.id === condition.ifFalseDependency.id),
            'ifFalseDependency was null',
          ),
          ifTrueBundles,
          ifFalseBundles,
          ifTrueAssetId: this.#graph.getAssetPublicId(trueAsset),
          ifFalseAssetId: this.#graph.getAssetPublicId(falseAsset),
        });
      }
    }

    return conditions;
  }

  // This is used to generate information for building a manifest that can
  // be used by a webserver to understand which conditions are used by which bundles,
  // and which bundles those conditions require depending on what they evaluate to.
  getConditionalBundleMapping(): Map<
    TBundle,
    Map<
      string,
      {|
        ifTrueBundles: Array<TBundle>,
        ifFalseBundles: Array<TBundle>,
      |},
    >,
  > {
    let bundleConditions = new Map();

    // Convert the internal references in conditions to public API references
    for (const cond of this.#graph._conditions.values()) {
      let assets = Array.from(cond.assets).map((asset) =>
        nullthrows(this.getAssetById(asset.id)),
      );
      let bundles = new Set<TBundle>();
      let ifTrueBundles = [];
      let ifFalseBundles = [];
      for (const asset of assets) {
        const bundlesWithAsset = this.getBundlesWithAsset(asset);
        for (const bundle of bundlesWithAsset) {
          bundles.add(bundle);
        }
        const assetDeps = this.getDependencies(asset);
        const depToBundles = (dep) => {
          const publicDep = nullthrows(
            assetDeps.find((assetDep) => dep.id === assetDep.id),
          );
          const resolved = this.resolveAsyncDependency(publicDep);
          if (!resolved) {
            // If there's no async dependency, don't list it as required
            return [];
          }
          invariant(resolved.type === 'bundle_group');
          return this.getBundlesInBundleGroup(resolved.value);
        };
        ifTrueBundles.push(...depToBundles(cond.ifTrueDependency));
        ifFalseBundles.push(...depToBundles(cond.ifFalseDependency));
      }

      for (let bundle of bundles) {
        const conditions = bundleConditions.get(bundle) ?? new Map();

        conditions.set(cond.key, {
          ifTrueBundles,
          ifFalseBundles,
        });

        bundleConditions.set(bundle, conditions);
      }
    }

    return bundleConditions;
  }

  getReferencedConditionalBundles(bundle: IBundle): Array<TBundle> {
    return this.#graph
      .getReferencedConditionalBundles(bundleToInternalBundle(bundle))
      .map((result) => this.#createBundle(result, this.#graph, this.#options));
  }
}
