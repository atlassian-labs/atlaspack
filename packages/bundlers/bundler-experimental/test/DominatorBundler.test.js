// @flow strict-local

import assert from 'assert';
import {EdgeContentGraph} from '../src/DominatorBundler/EdgeContentGraph';
import type {Asset, Dependency, Target} from '@atlaspack/types';
import type {
  AssetNode,
  PackagedDominatorGraph,
  SimpleBundle,
  SimpleBundleGroup,
} from '../src';
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
    const entryAsset = makeAsset({filePath: 'entry'});
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
    const entryAsset = makeAsset({filePath: 'entry'});
    const asyncAsset = makeAsset({filePath: 'async'});
    const target = makeTarget({});
    const entryDep = makeDependency({target});
    const asyncDependency = makeDependency({
      priority: 'parallel',
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
    packages.addWeightedEdge(root, asyncId, 1, [asyncDependency]);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    entryDependenciesByAsset.set(asyncId, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();
    asyncDependenciesByAsset.set(asyncId, new Set([asyncNode]));

    const bundleGroupsByEntryDep = new DefaultMap(() => new Map());
    assert.equal(bundleGroupsByEntryDep.get(entryDep.target).size, 0);
    const rootResult = getOrCreateBundleGroupsForNode(
      bundleGroupsByEntryDep,
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
      asset,
      assetNode,
    );
    assert.equal(rootResult.size, 1);
    assert.equal(bundleGroupsByEntryDep.get(entryDep.target).size, 1);

    const result = Array.from(
      getOrCreateBundleGroupsForNode(
        bundleGroupsByEntryDep,
        packages,
        entryDependenciesByAsset,
        asyncDependenciesByAsset,
        asyncId,
        asyncNode,
      ),
    );
    assert.equal(bundleGroupsByEntryDep.get(entryDep.target).size, 2);

    assert.strictEqual(result[0].entryDep, asyncDependency);
    assert.deepStrictEqual(result, [
      {
        entryDep: asyncDependency,
        target: assetNode.target,
        bundles: [],
      },
    ]);
  });

  it('will not return a bundle group for a type change node', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const htmlAsset = makeAsset({filePath: 'entry.html'});
    const jsAsset = makeAsset({filePath: 'index.js'});
    const target = makeTarget({});
    const entryDep = makeDependency({target, isEntry: true});
    const jsDependency = makeDependency({priority: 'sync'});
    const assetNode = {
      type: 'asset',
      id: 'html',
      asset: htmlAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
      isRoot: true,
    };
    const asset = packages.addNodeByContentKey('asset', assetNode);
    packages.addEdge(root, asset);

    const jsNode = {
      type: 'asset',
      id: 'js',
      asset: jsAsset,
      entryDependency: entryDep,
      target: null,
      isEntryNode: false,
      isRoot: true,
    };
    const jsId = packages.addNodeByContentKey('js', jsNode);
    packages.addWeightedEdge(root, jsId, 1, [jsDependency]);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    entryDependenciesByAsset.set(jsId, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();

    const bundleGroupsByEntryDep = new DefaultMap(() => new Map());
    assert.equal(bundleGroupsByEntryDep.get(entryDep.target).size, 0);
    getOrCreateBundleGroupsForNode(
      bundleGroupsByEntryDep,
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
      asset,
      assetNode,
    );
    assert.equal(bundleGroupsByEntryDep.get(entryDep.target).size, 1);
    const result = getOrCreateBundleGroupsForNode(
      bundleGroupsByEntryDep,
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
      jsId,
      jsNode,
    );
    assert.equal(bundleGroupsByEntryDep.get(entryDep.target).size, 1);

    assert.deepEqual(Array.from(result), [
      {
        entryDep: entryDep,
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
    const entryAsset = makeAsset({filePath: 'entry'});
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

    const expectedBundles: SimpleBundle[] = [
      {
        type: 'entry',
        assets: [entryAsset],
        options: {
          entryAsset,
          bundleBehavior: undefined,
          needsStableName: true,
          target,
        },
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
    const entryAsset = makeAsset({filePath: 'entry'});
    const asyncAssetValue = makeAsset({filePath: 'async'});
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
    packages.addWeightedEdge(root, asyncAsset, 1, [asyncDependency]);

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

    const expectedBundles: SimpleBundle[] = [
      {
        type: 'entry',
        assets: [entryAsset],
        options: {
          entryAsset,
          bundleBehavior: undefined,
          needsStableName: true,
          target,
        },
      },
    ];
    const expectedAsyncBundles: SimpleBundle[] = [
      {
        type: 'entry',
        assets: [asyncAssetValue],
        options: {
          entryAsset: asyncAssetValue,
          bundleBehavior: undefined,
          needsStableName: false,
          target,
        },
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

  it.skip('can plan a graph with a shared bundle', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({filePath: 'entry'});
    const sharedAssetValue = makeAsset({filePath: 'shared'});
    const target = makeTarget({});
    const entryDep = makeDependency({
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
    const sharedAssetNode = {
      type: 'asset',
      id: 'shared-asset',
      asset: sharedAssetValue,
      entryDependency: null,
      target: null,
      isEntryNode: false,
      isRoot: false,
    };
    const sharedAsset = packages.addNodeByContentKey(
      'shared-asset',
      sharedAssetNode,
    );
    packages.addEdge(root, sharedAsset);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    entryDependenciesByAsset.set(sharedAsset, new Set([assetNode]));
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
      {
        entryAsset: sharedAssetValue,
        needsStableName: false,
        target,
        assets: [sharedAssetValue],
      },
    ];
    assert.deepStrictEqual(result.bundles, [...expectedBundles]);
    assert.deepStrictEqual(result.bundleGroups, [
      {
        entryDep,
        target,
        bundles: expectedBundles,
      },
    ]);
  });

  it('can plan a graph with a shared bundle used on async roots', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({filePath: 'entry'});
    const page1Asset = makeAsset({filePath: 'page1'});
    const page2Asset = makeAsset({filePath: 'page2'});
    const sharedAssetValue = makeAsset({filePath: 'shared'});
    const target = makeTarget({});
    const entryDep = makeDependency({
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
    const page1Node = {
      type: 'asset',
      id: 'page1',
      asset: page1Asset,
      entryDependency: null,
      target: null,
      isEntryNode: false,
      isRoot: true,
    };
    const page1 = packages.addNodeByContentKey('page1', page1Node);
    const page1Dependency = makeDependency({id: 'index-to-page1'});
    packages.addWeightedEdge(root, page1, 1, [page1Dependency]);
    const page2Node = {
      type: 'asset',
      id: 'page2',
      asset: page2Asset,
      entryDependency: null,
      target: null,
      isEntryNode: false,
      isRoot: true,
    };
    const page2 = packages.addNodeByContentKey('page2', page2Node);
    const page2Dependency = makeDependency({id: 'index-to-page2'});
    packages.addWeightedEdge(root, page2, 1, [page2Dependency]);
    const sharedAssetNode = {
      type: 'asset',
      id: 'shared-asset',
      asset: sharedAssetValue,
      entryDependency: null,
      target: null,
      isEntryNode: false,
      isRoot: false,
    };
    const sharedAsset = packages.addNodeByContentKey(
      'shared-asset',
      sharedAssetNode,
    );
    packages.addEdge(root, sharedAsset);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, new Set([assetNode]));
    entryDependenciesByAsset.set(sharedAsset, new Set([assetNode]));
    entryDependenciesByAsset.set(page1, new Set([assetNode]));
    entryDependenciesByAsset.set(page2, new Set([assetNode]));
    const asyncDependenciesByAsset = new Map();
    asyncDependenciesByAsset.set(sharedAsset, new Set([page1Node, page2Node]));
    asyncDependenciesByAsset.set(page1, new Set([page1Node]));
    asyncDependenciesByAsset.set(page2, new Set([page2Node]));

    const result = planBundleGraph(
      packages,
      entryDependenciesByAsset,
      asyncDependenciesByAsset,
    );

    const expectedBundles: SimpleBundle[] = [
      {
        type: 'entry',
        assets: [entryAsset],
        options: {
          bundleBehavior: undefined,
          entryAsset,
          needsStableName: true,
          target,
        },
      },
      {
        type: 'entry',
        assets: [page1Asset],
        options: {
          entryAsset: page1Asset,
          bundleBehavior: undefined,
          needsStableName: false,
          target,
        },
      },
      {
        type: 'entry',
        assets: [page2Asset],
        options: {
          bundleBehavior: undefined,
          entryAsset: page2Asset,
          needsStableName: false,
          target,
        },
      },
      {
        type: 'entry',
        assets: [sharedAssetValue],
        options: {
          bundleBehavior: undefined,
          entryAsset: sharedAssetValue,
          needsStableName: false,
          target,
        },
      },
    ];
    assert.deepStrictEqual(result.bundles, [...expectedBundles]);
    const expectedBundleGroups: SimpleBundleGroup[] = [
      {
        entryDep,
        target,
        bundles: [expectedBundles[0]],
      },
      {
        entryDep: page1Dependency,
        target,
        bundles: [expectedBundles[1], expectedBundles[3]],
      },
      {
        entryDep: page2Dependency,
        target,
        bundles: [expectedBundles[2], expectedBundles[3]],
      },
    ];
    assert.deepStrictEqual(result.bundleGroups, expectedBundleGroups);
  });
});
