// @flow strict-local

import sinon from 'sinon';
import assert from 'assert';
import {EdgeContentGraph} from '../src/DominatorBundler/EdgeContentGraph';
import type {Asset, Dependency, Target} from '@atlaspack/types';
import type {AssetNode, PackagedDominatorGraph} from '../src';
import {addNodeToBundle, planBundleGraph} from '../src';

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
    const mockBundleGraph = {
      addAssetToBundle: sinon.spy(),
    };
    const mockBundle = {};
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
    addNodeToBundle(packages, mockBundleGraph, mockBundle, assetId);

    assert(mockBundleGraph.addAssetToBundle.calledOnce);
    assert(mockBundleGraph.addAssetToBundle.calledWith(mockAsset, mockBundle));
  });

  it('adds a tree of asset nodes into the bundle', () => {
    const mockBundleGraph = {
      addAssetToBundle: sinon.spy(),
    };
    const mockBundle = {};
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
    addNodeToBundle(packages, mockBundleGraph, mockBundle, parentAsset);

    assert(mockBundleGraph.addAssetToBundle.calledWith(mockAsset, mockBundle));
    assert(
      mockBundleGraph.addAssetToBundle.calledWith(mockChildAsset, mockBundle),
    );
  });

  it('adds a package node to the bundle', () => {
    const mockBundleGraph = {
      addAssetToBundle: sinon.spy(),
    };
    const mockBundle = {};
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
    addNodeToBundle(packages, mockBundleGraph, mockBundle, parentPackage);

    assert(
      mockBundleGraph.addAssetToBundle.calledWith(mockChildAsset, mockBundle),
    );
    assert(
      mockBundleGraph.addAssetToBundle.calledWith(mockNestedChild, mockBundle),
    );
    assert(
      mockBundleGraph.addAssetToBundle.calledWith(
        mockSecondTopLevelChild,
        mockBundle,
      ),
    );
  });

  it('adds strongly connected components to a bundle', () => {
    const mockBundleGraph = {
      addAssetToBundle: sinon.spy(),
    };
    const mockBundle = {};

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
    addNodeToBundle(packages, mockBundleGraph, mockBundle, childAsset);

    assert(
      mockBundleGraph.addAssetToBundle.calledWith(mockChildAsset, mockBundle),
    );
    assert(mockBundleGraph.addAssetToBundle.calledWith(sccChild1, mockBundle));
    assert(mockBundleGraph.addAssetToBundle.calledWith(sccChild2, mockBundle));
    assert(
      mockBundleGraph.addAssetToBundle.calledWith(mockNestedChild, mockBundle),
    );
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
    });
  });

  it('can plan a graph with a single asset node', () => {
    const packages: PackagedDominatorGraph = new EdgeContentGraph();
    const root = packages.addNodeByContentKey('root', 'root');
    packages.setRootNodeId(root);
    const entryAsset = makeAsset({});
    const target = makeTarget({});
    const entryDep = makeDependency({target});
    const asset = packages.addNodeByContentKey('asset', {
      type: 'asset',
      id: 'asset',
      asset: entryAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
    });
    packages.addEdge(root, asset);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, [
      {
        assetNode: {type: 'asset', id: 'asset', asset: entryAsset},
        entryDependency: entryDep,
      },
    ]);
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
      },
    ];

    assert.deepStrictEqual(result, {
      bundles: expectedBundles,
      bundleGroups: [
        {
          entryDep,
          target,
          bundles: expectedBundles,
        },
      ],
    });
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
    const assetNode = {
      type: 'asset',
      id: 'asset',
      asset: entryAsset,
      entryDependency: entryDep,
      target,
      isEntryNode: true,
    };
    const asset = packages.addNodeByContentKey('asset', assetNode);
    packages.addEdge(root, asset);
    const asyncAsset = packages.addNodeByContentKey('async-asset', {
      type: 'asset',
      id: 'async-asset',
      asset: asyncAssetValue,
      entryDependency: null,
      target: null,
      isEntryNode: false,
    });
    packages.addEdge(root, asyncAsset);

    const entryDependenciesByAsset = new Map();
    entryDependenciesByAsset.set(asset, [assetNode]);
    entryDependenciesByAsset.set(asyncAsset, [assetNode]);
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
      },
      {
        entryAsset: asyncAssetValue,
        needsStableName: false,
        target,
      },
    ];
    assert.deepStrictEqual(result, {
      bundles: expectedBundles,
      bundleGroups: [
        {
          entryDep,
          target,
          bundles: expectedBundles,
        },
      ],
    });
  });
});
