// @flow strict-local

import assert from 'assert';
import {EdgeContentGraph} from '../src/DominatorBundler/EdgeContentGraph';
import type {Asset, Dependency, Target} from '@atlaspack/types';
import type {AssetNode, PackagedDominatorGraph} from '../src';
import {
  addNodeToBundle,
  getOrCreateBundleGroupsForNode,
  planBundleGraph,
} from '../src';
import {DefaultMap} from '@atlaspack/utils';

// $FlowFixMe
const makeDependency = (obj: mixed): Dependency => (obj: any);

// $FlowFixMe
const makeTarget = (obj: mixed): Target => (obj: any);

// $FlowFixMe
const makeAsset = (obj: mixed): Asset => (obj: any);

// $FlowFixMe
const makeAssetNode = (node: mixed): AssetNode => (node: any);

describe('addNodeToBundle', () => {
  it('adds asset nodes into the bundle', () => {
    const mockBundle = {
      assets: [],
    };
    const mockAsset = {};
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNode('root');
    packages.setRootNodeId(root);

    const assetId = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockAsset,
      }),
    );
    packages.addEdge(root, assetId);

    // $FlowFixMe
    addNodeToBundle(packages, mockBundle, assetId);

    assert.deepStrictEqual(mockBundle.assets, [mockAsset]);
  });

  it('adds a tree of asset nodes into the bundle', () => {
    const mockBundle = {
      assets: [],
    };
    const mockAsset = {};
    const mockChildAsset = {};
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNode('root');
    packages.setRootNodeId(root);

    const parentAsset = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockAsset,
      }),
    );
    packages.addEdge(root, parentAsset);
    const childAsset = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockChildAsset,
      }),
    );
    packages.addEdge(parentAsset, childAsset);

    // $FlowFixMe
    addNodeToBundle(packages, mockBundle, parentAsset);

    assert.deepStrictEqual(mockBundle.assets, [mockAsset, mockChildAsset]);
  });

  it('adds a package node to the bundle', () => {
    const mockBundle = {
      assets: [],
    };
    const mockChildAsset = {id: 'child'};
    const mockNestedChild = {id: 'nested'};
    const mockSecondTopLevelChild = {id: 'second-child'};
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNode('root');
    packages.setRootNodeId(root);

    const parentPackage = packages.addNode(
      ({
        type: 'package',
        // $FlowFixMe
      }: any),
    );
    packages.addEdge(root, parentPackage);
    const childAsset = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockChildAsset,
      }),
    );
    packages.addEdge(parentPackage, childAsset);
    const nestedChild = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockNestedChild,
      }),
    );
    packages.addEdge(childAsset, nestedChild);
    const secondChild = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockSecondTopLevelChild,
      }),
    );
    packages.addEdge(parentPackage, secondChild);

    // $FlowFixMe
    addNodeToBundle(packages, mockBundle, parentPackage);

    assert.deepStrictEqual(mockBundle.assets, [
      mockChildAsset,
      mockNestedChild,
      mockSecondTopLevelChild,
    ]);
  });

  it('adds strongly connected components to a bundle', () => {
    const mockBundle = {
      assets: [],
    };

    const mockChildAsset = makeAsset({id: 'child'});
    const mockNestedChild = makeAsset({id: 'nested'});
    const sccChild1 = makeAsset({id: 'scc-child1'});
    const sccChild2 = makeAsset({id: 'scc-child2'});

    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNode('root');
    packages.setRootNodeId(root);

    const childAsset = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockChildAsset,
      }),
    );
    packages.addEdge(root, childAsset);

    const stronglyConnectedComponent = packages.addNode({
      type: 'StronglyConnectedComponent',
      id: 'scc',
      nodeIds: [],
      values: [
        makeAssetNode({
          type: 'asset',
          asset: sccChild1,
        }),
        makeAssetNode({
          type: 'asset',
          asset: sccChild2,
        }),
      ],
    });
    packages.addEdge(childAsset, stronglyConnectedComponent);

    const nested = packages.addNode(
      makeAssetNode({
        type: 'asset',
        asset: mockNestedChild,
      }),
    );
    packages.addEdge(stronglyConnectedComponent, nested);

    // $FlowFixMe
    addNodeToBundle(packages, mockBundle, childAsset);

    assert.deepStrictEqual(mockBundle.assets, [
      mockChildAsset,
      sccChild1,
      sccChild2,
      mockNestedChild,
    ]);
  });
});

describe('getOrCreateBundleGroupsForNode', () => {
  it('will return a bundle group for an entry-point node', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({});
    const target = makeTarget({});
    const entryDep = makeDependency({target});
    const assetNode = {
      type: 'asset',
      id: 'asset',
      asset: entryAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
      isRoot: true,
    };
    const asset = packages.addNodeByContentKey('asset', assetNode);
    packages.addEdge(root, asset);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();

    const bundleGroupsByEntryDep = new DefaultMap(() => new Map());
    const result = getOrCreateBundleGroupsForNode(
      bundleGroupsByEntryDep,
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
      asset,
      assetNode,
    );

    assert.deepEqual(Array.from(result), [
      {
        entryDep: assetNode.entryDependency,
        target: assetNode.target,
        bundles: [],
      },
    ]);
  });

  it('will return a bundle group for an async node', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({});
    const asyncAsset = makeAsset({});
    const target = makeTarget({});
    const entryDep = makeDependency({target});
    const asyncDependency = makeDependency({});
    const assetNode = {
      type: 'asset',
      id: 'asset',
      asset: entryAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
      isRoot: true,
    };
    const asset = packages.addNodeByContentKey('asset', assetNode);
    packages.addEdge(root, asset);

    const asyncNode = {
      type: 'asset',
      id: 'async',
      asset: asyncAsset,
      entryDependency: null,
      target: null,
      isEntryNode: false,
      isRoot: true,
    };
    const asyncId = packages.addNodeByContentKey('async', asyncNode);
    packages.addWeightedEdge(root, asyncId, 1, asyncDependency);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    entryDependenciesByAsset.set(asyncId, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();
    asyncDependenciesByAsset.set(asyncId, new Set([asyncNode]));

    const bundleGroupsByEntryDep = new DefaultMap(() => new Map());
    const result = getOrCreateBundleGroupsForNode(
      bundleGroupsByEntryDep,
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
      asyncId,
      asyncNode,
    );

    assert.deepEqual(Array.from(result), [
      {
        entryDep: asyncDependency,
        target: assetNode.target,
        bundles: [],
      },
    ]);
  });
});

describe('planBundleGraph', () => {
  it('can plan an empty graph', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);

    const entryDependenciesByAsset = new Map();
    const asyncDependenciesByAsset = new Map();
    const result = planBundleGraph(
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
    );

    assert.deepEqual(result, {
      bundles: [],
      bundleGroups: [],
      bundlesByPackageContentKey: new Map(),
    });
  });

  it('can plan a graph with a single asset node', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({});
    const target = makeTarget({});
    const entryDep = makeDependency({target});
    const assetNode = {
      type: 'asset',
      id: 'asset',
      asset: entryAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
      isRoot: true,
    };
    const asset = packages.addNodeByContentKey('asset', assetNode);
    packages.addEdge(root, asset);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();

    const result = planBundleGraph(
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
    );

    const expectedBundles = [
      {
        entryAsset,
        needsStableName: true,
        target,
        assets: [entryAsset],
      },
    ];

    assert.deepStrictEqual(result.bundles, expectedBundles);
    assert.deepStrictEqual(
      result.bundlesByPackageContentKey,
      new Map([['asset', expectedBundles[0]]]),
    );
    assert.deepStrictEqual(result.bundleGroups, [
      {
        entryDep,
        target,
        bundles: expectedBundles,
      },
    ]);
  });

  it('can plan a graph with two async dependant bundles', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({});
    const asyncAssetValue = makeAsset({});
    const target = makeTarget({});
    const entryDep = makeDependency({
      target,
    });
    const asyncDependency = makeDependency({
      target,
    });
    const assetNode = {
      type: 'asset',
      id: 'asset',
      asset: entryAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
      isRoot: true,
    };
    const asset = packages.addNodeByContentKey('asset', assetNode);
    packages.addEdge(root, asset);
    const asyncAssetNode = {
      type: 'asset',
      id: 'async-asset',
      asset: asyncAssetValue,
      entryDependency: null,
      target: null,
      isEntryNode: false,
      isRoot: true,
    };
    const asyncAsset = packages.addNodeByContentKey(
      'async-asset',
      asyncAssetNode,
    );
    packages.addWeightedEdge(root, asyncAsset, 1, asyncDependency);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    entryDependenciesByAsset.set(asyncAsset, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();
    asyncDependenciesByAsset.set(asyncAsset, new Set([asyncAssetNode]));

    const result = planBundleGraph(
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
    );

    const expectedBundles = [
      {
        entryAsset,
        needsStableName: true,
        target,
        assets: [entryAsset],
      },
    ];
    const expectedAsyncBundles = [
      {
        entryAsset: asyncAssetValue,
        needsStableName: false,
        target,
        assets: [asyncAssetValue],
      },
    ];
    assert.deepStrictEqual(result.bundles, [
      ...expectedBundles,
      ...expectedAsyncBundles,
    ]);
    assert.deepStrictEqual(result.bundleGroups, [
      {
        entryDep,
        target,
        bundles: expectedBundles,
      },
      {
        entryDep: asyncDependency,
        target,
        bundles: expectedAsyncBundles,
      },
    ]);
  });
});
