// @flow strict-local

import sinon from 'sinon';
import assert from 'assert';
import {ContentGraph} from '@atlaspack/graph';
import type {PackagedDominatorGraph} from '../src';
import {addNodeToBundle} from '../src';

describe('addNodeToBundle', () => {
  it('adds asset nodes into the bundle', () => {
    const mockBundleGraph = {
      addAssetToBundle: sinon.spy(),
    };
    const mockBundle = {};
    const mockAsset = {};
    const packages: PackagedDominatorGraph = new ContentGraph();
    const root = packages.addNode('root', 'root');
    packages.setRootNodeId(root);

    const assetId = packages.addNode({
      type: 'asset',
      asset: mockAsset,
    });
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
    const packages: PackagedDominatorGraph = new ContentGraph();
    const root = packages.addNode('root', 'root');
    packages.setRootNodeId(root);

    const parentAsset = packages.addNode({
      type: 'asset',
      asset: mockAsset,
    });
    packages.addEdge(root, parentAsset);
    const childAsset = packages.addNode({
      type: 'asset',
      asset: mockChildAsset,
    });
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
    const packages: PackagedDominatorGraph = new ContentGraph();
    const root = packages.addNode('root', 'root');
    packages.setRootNodeId(root);

    const parentPackage = packages.addNode({
      type: 'package',
    });
    packages.addEdge(root, parentPackage);
    const childAsset = packages.addNode({
      type: 'asset',
      asset: mockChildAsset,
    });
    packages.addEdge(parentPackage, childAsset);
    const nestedChild = packages.addNode({
      type: 'asset',
      asset: mockNestedChild,
    });
    packages.addEdge(childAsset, nestedChild);
    const secondChild = packages.addNode({
      type: 'asset',
      asset: mockSecondTopLevelChild,
    });
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
    const mockChildAsset = {id: 'child'};
    const mockNestedChild = {id: 'nested'};
    const sccChild1 = {id: 'scc-child1'};
    const sccChild2 = {id: 'scc-child2'};
    const packages: PackagedDominatorGraph = new ContentGraph();
    const root = packages.addNode('root', 'root');
    packages.setRootNodeId(root);

    const childAsset = packages.addNode({
      type: 'asset',
      asset: mockChildAsset,
    });
    packages.addEdge(root, childAsset);

    const stronglyConnectedComponent = packages.addNode({
      type: 'StronglyConnectedComponent',
      values: [
        {
          type: 'asset',
          asset: sccChild1,
        },
        {
          type: 'asset',
          asset: sccChild2,
        },
      ],
    });
    packages.addEdge(childAsset, stronglyConnectedComponent);

    const nested = packages.addNode({
      type: 'asset',
      asset: mockNestedChild,
    });
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
