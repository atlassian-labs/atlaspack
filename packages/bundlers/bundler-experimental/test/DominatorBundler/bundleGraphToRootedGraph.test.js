// @flow strict-local

import {getParcelOptions, overlayFS, workerFarm} from '@atlaspack/test-utils';
import nullthrows from 'nullthrows';
import * as path from 'path';
import assert from 'assert';
import MutableBundleGraph from '@atlaspack/core/src/public/MutableBundleGraph';
import resolveOptions from '@atlaspack/core/src/resolveOptions';
import AssetGraph from '@atlaspack/core/src/AssetGraph';
import BundleGraph from '@atlaspack/core/src/BundleGraph';
import type {AssetGraphNode} from '@atlaspack/core/src/types';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import {dotTest, setupBundlerTest} from '../test-utils';
import {rootedGraphToDot} from '../graphviz/GraphvizUtils';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import {toProjectPath} from '@atlaspack/core/src/projectPath';

function encodeHex(str: string): string {
  return Buffer.from(str).toString('hex');
}

// $FlowFixMe
function makeDependencyNode(dependency: any): AssetGraphNode {
  // $FlowFixMe
  return {
    id: dependency.id,
    type: 'dependency',
    value: dependency,
    hasDeferred: false,
  };
}

// $FlowFixMe
function makeAssetNode(asset: any): AssetGraphNode {
  // $FlowFixMe
  return {
    id: asset.id,
    type: 'asset',
    value: asset,
  };
}

describe('bundleGraphToRootedGraph', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  it('returns a simple graph with a single root', async () => {
    const entryPath = path.join(__dirname, 'test/test.js');
    await fixtureFromGraph(path.dirname(entryPath), overlayFS, [
      asset('test.js', ['dependency.js']),
      asset('dependency.js'),
    ]);

    const {mutableBundleGraph} = await setupBundlerTest(entryPath);
    const rootGraph = bundleGraphToRootedGraph(mutableBundleGraph);

    const rootNode = rootGraph.getNodeIdByContentKey('root');
    const assetIdsByPath = new Map();
    rootGraph.traverse((node) => {
      if (node !== rootNode) {
        const assetNode = rootGraph.getNode(node);
        if (!assetNode || typeof assetNode === 'string') {
          throw new Error('Asset not found');
        }
        assetIdsByPath.set(
          path.basename(assetNode.asset.filePath),
          assetNode.id,
        );
      }
    }, rootNode);

    const getConnections = (contentKey: string) => {
      const node = rootGraph.getNodeIdByContentKey(contentKey);
      return rootGraph
        .getNodeIdsConnectedFrom(node)
        .map((nodeId) => {
          const node = rootGraph.getNode(nodeId);
          if (!node || typeof node === 'string') throw new Error('root cycle');
          return path.basename(node.asset.filePath);
        })
        .filter((path) => !path.includes('esmodule-helpers.js'));
    };

    assert.deepEqual(getConnections('root'), ['test.js']);
    assert.deepEqual(
      getConnections(nullthrows(assetIdsByPath.get('test.js'))),
      ['dependency.js'],
    );
    assert.deepEqual(
      getConnections(nullthrows(assetIdsByPath.get('dependency.js'))),
      [],
    );
  });

  dotTest(
    __filename,
    'converts the bundle graph into a simplified representation',
    async () => {
      const entryDir = 'test';
      await fixtureFromGraph(entryDir, overlayFS, [
        asset('page1.js', ['library1.js']),
        asset('page2.js', ['library2.js']),
        asset('library1.js', ['library3.js']),
        asset('library2.js', ['library3.js']),
        asset('library3.js'),
      ]);
      const {mutableBundleGraph} = await setupBundlerTest([
        path.join(entryDir, 'page1.js'),
        path.join(entryDir, 'page2.js'),
      ]);

      const simplifiedGraph = bundleGraphToRootedGraph(mutableBundleGraph);
      const dot = rootedGraphToDot(
        entryDir,
        simplifiedGraph,
        'Simplified Graph',
        'simplified_graph',
      );
      assert.equal(
        dot,
        `
digraph simplified_graph {
  labelloc="t";
  label="Simplified Graph";

  "root";
  "root" -> "page1.js";
  "root" -> "page2.js";
  "library1.js";
  "library2.js";
  "library3.js";
  "page1.js";
  "page2.js";

  "library1.js" -> "library3.js";
  "library2.js" -> "library3.js";
  "page1.js" -> "library1.js";
  "page2.js" -> "library2.js";
}
        `.trim(),
      );

      return [
        {
          label: 'simplifiedGraph',
          dot,
        },
      ];
    },
  );

  dotTest(__filename, 'async dependencies are linked to the root', async () => {
    const entryDir = 'test';
    await fixtureFromGraph(entryDir, overlayFS, [
      asset('page1.js', [{to: 'library1.js', type: 'async'}, 'library2.js']),
      asset('library1.js'),
      asset('library2.js'),
    ]);
    const {mutableBundleGraph} = await setupBundlerTest([
      path.join(entryDir, 'page1.js'),
    ]);

    const simplifiedGraph = bundleGraphToRootedGraph(mutableBundleGraph);
    const dot = rootedGraphToDot(
      entryDir,
      simplifiedGraph,
      'Simplified Graph',
      'simplified_graph',
    );
    assert.equal(
      dot,
      `
digraph simplified_graph {
  labelloc="t";
  label="Simplified Graph";

  "root";
  "root" -> "library1.js";
  "root" -> "page1.js";
  "library1.js";
  "library2.js";
  "page1.js";

  "page1.js" -> "library2.js";
}
        `.trim(),
    );

    return [
      {
        label: 'simplifiedGraph',
        dot,
      },
    ];
  });

  it('dependencies of different types are linked to the root', async () => {
    const options = getParcelOptions('/test/index.js', {
      inputFS: overlayFS,
      defaultConfig: path.join(__dirname, 'atlaspack-config.json'),
    });
    const resolvedOptions = await resolveOptions(options);
    resolvedOptions.projectRoot = '/test';

    const assetGraph = new AssetGraph();
    const entry = assetGraph.addNode(
      makeDependencyNode({
        isEntry: true,
      }),
    );
    const entryAsset = assetGraph.addNode(
      makeAssetNode({
        id: encodeHex('asset-1'),
        type: 'js',
        filePath: '/test/index.js',
      }),
    );
    const dependency = assetGraph.addNode(
      makeDependencyNode({
        id: 'child-dependency',
        isEntry: false,
        sourceAssetType: 'js',
      }),
    );
    const childAsset = assetGraph.addNode({
      id: encodeHex('asset-of-different-type'),
      type: 'asset',
      // $FlowFixMe
      value: {
        id: encodeHex('asset-of-different-type'),
        type: 'png',
        filePath: toProjectPath('/test', 'child.png'),
      },
      usedSymbols: new Set(),
      hasDeferred: false,
      usedSymbolsDownDirty: false,
      usedSymbolsUpDirty: false,
      requested: true,
    });
    assetGraph.addEdge(nullthrows(assetGraph.rootNodeId), entry);
    assetGraph.addEdge(entry, entryAsset);
    assetGraph.addEdge(entryAsset, dependency);
    assetGraph.addEdge(dependency, childAsset);

    const bundleGraph = BundleGraph.fromAssetGraph(assetGraph, false);
    const mutableBundleGraph = new MutableBundleGraph(
      bundleGraph,
      resolvedOptions,
    );

    const simplifiedGraph = bundleGraphToRootedGraph(mutableBundleGraph);

    const dot = rootedGraphToDot(
      '/test',
      simplifiedGraph,
      'Simplified Graph',
      'simplified_graph',
    );
    assert.equal(
      dot,
      `
digraph simplified_graph {
  labelloc="t";
  label="Simplified Graph";

  "root";
  "root" -> "child.png";
  "root" -> "index.js";
  "child.png";
  "index.js";

}
        `.trim(),
    );
  });
});
