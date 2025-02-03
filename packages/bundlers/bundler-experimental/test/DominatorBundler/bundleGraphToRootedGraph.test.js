// @flow strict-local

import {
  fsFixture,
  getParcelOptions,
  overlayFS,
  workerFarm,
} from '@atlaspack/test-utils';
import nullthrows from 'nullthrows';
import * as path from 'path';
import assert from 'assert';
import type {NodeId} from '@atlaspack/graph';
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
import type {AssetNode} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import invariant from 'graphql/jsutils/invariant';

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

function getNodeIdsByPath(simplifiedGraph): Map<string, NodeId> {
  const nodeIdsByPath = new Map();
  simplifiedGraph.traverse((nodeId) => {
    const node = simplifiedGraph.getNode(nodeId);
    if (node == null || typeof node === 'string') {
      return;
    }
    nodeIdsByPath.set(path.basename(node.asset.filePath), nodeId);
  });
  return nodeIdsByPath;
}

function getContentKeysByPath(simplifiedGraph): Map<string, string> {
  const nodeIdsByPath = new Map();
  simplifiedGraph.traverse((nodeId) => {
    const node = simplifiedGraph.getNode(nodeId);
    if (node == null || typeof node === 'string') {
      return;
    }
    nodeIdsByPath.set(
      path.basename(node.asset.filePath),
      simplifiedGraph.getContentKeyByNodeId(nodeId),
    );
  });
  return nodeIdsByPath;
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
    const rootGraph = bundleGraphToRootedGraph(mutableBundleGraph).getGraph();

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

      const simplifiedGraph =
        bundleGraphToRootedGraph(mutableBundleGraph).getGraph();
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

    const result = bundleGraphToRootedGraph(mutableBundleGraph);

    const simplifiedGraph = result.getGraph();
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

    const contentKeysByPath = getContentKeysByPath(simplifiedGraph);
    const bundleReferences = result.getBundleReferences();
    const page1Key = contentKeysByPath.get('page1.js') ?? -1;
    const library1Key = contentKeysByPath.get('library1.js') ?? -1;
    const library2Key = contentKeysByPath.get('library2.js') ?? -1;
    assert.deepEqual(
      bundleReferences.get(page1Key)?.map((r) => r.assetContentKey) ?? [],
      [library2Key],
    );
    assert.deepEqual(
      bundleReferences.get(library1Key)?.map((r) => r.assetContentKey) ?? [],
      [],
    );
    assert.deepEqual(
      bundleReferences.get(library2Key)?.map((r) => r.assetContentKey) ?? [],
      [],
    );

    return [
      {
        label: 'simplifiedGraph',
        dot,
      },
    ];
  });

  it('HTML dependencies get split properly', async () => {
    const entryDir = path.join(__dirname, 'html-dependencies');

    await fsFixture(overlayFS, __dirname)`
    html-dependencies
      index.html:
        <script src="./page1.js"></script>

      page1.js:
        console.log(1);
    `;
    const {mutableBundleGraph} = await setupBundlerTest([
      path.join(entryDir, 'index.html'),
    ]);

    const result = bundleGraphToRootedGraph(mutableBundleGraph);
    const simplifiedGraph = result.getGraph();

    const root = simplifiedGraph.getNodeIdByContentKey('root');
    const rootNodes: AssetNode[] = simplifiedGraph
      .getNodeIdsConnectedFrom(root)
      .map((nodeId) => {
        const node = simplifiedGraph.getNode(nodeId);
        invariant(node != null && typeof node !== 'string');
        return node;
      });

    // 1 HTML + 2 JS targets
    assert.equal(rootNodes.length, 2);
    const htmlNode = rootNodes.find((node) => node.asset.type === 'html');
    invariant(htmlNode != null);
    invariant(typeof htmlNode !== 'string');
    assert.equal(htmlNode.type, 'asset');
    assert.equal(
      htmlNode.entryDependency?.specifier,
      'packages/bundlers/bundler-experimental/test/DominatorBundler/html-dependencies/index.html',
    );
    const jsNode = rootNodes.find((node) => node.asset.type === 'js');
    invariant(jsNode != null);
    assert.equal(path.basename(jsNode.asset.filePath), 'page1.js');
    assert.strictEqual(jsNode.entryDependency, htmlNode.entryDependency);

    const dot = rootedGraphToDot(
      entryDir,
      simplifiedGraph,
      'Simplified Graph',
      'simplified_graph',
    );
    // page1.js is duplicated because of type=module
    assert.equal(
      dot,
      `
digraph simplified_graph {
  labelloc="t";
  label="Simplified Graph";

  "root";
  "root" -> "index.html";
  "root" -> "page1.js";
  "index.html";
  "page1.js";

  "index.html" -> "page1.js";
}
        `.trim(),
    );

    const contentKeysByPath = getContentKeysByPath(simplifiedGraph);
    const bundleReferences = result.getBundleReferences();
    const indexKey = contentKeysByPath.get('index.html') ?? -1;
    const page1Key = contentKeysByPath.get('page1.js') ?? -1;
    assert.deepEqual(
      bundleReferences.get(indexKey)?.map((r) => r.assetContentKey) ?? [],
      [page1Key],
    );
    assert.deepEqual(
      bundleReferences.get(page1Key)?.map((r) => r.assetContentKey) ?? [],
      [],
    );
  });

  it('image dependencies are linked to the root', async () => {
    const entryDir = path.join(__dirname, 'html-dependencies');

    await fsFixture(overlayFS, __dirname)`
    html-dependencies
      page1.js:
        const image = require('./image.png');;
        console.log(image);

      image.png:
        console.log(1);
    `;
    const {mutableBundleGraph} = await setupBundlerTest([
      path.join(entryDir, 'page1.js'),
    ]);

    const simplifiedGraph =
      bundleGraphToRootedGraph(mutableBundleGraph).getGraph();
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
  "root" -> "image.png";
  "root" -> "page1.js";
  "image.png";
  "page1.js";

  "page1.js" -> "image.png";
}
        `.trim(),
    );
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

    const result = bundleGraphToRootedGraph(mutableBundleGraph);
    const simplifiedGraph = result.getGraph();

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

  "index.js" -> "child.png";
}
        `.trim(),
    );

    const contentKeysByPath = getContentKeysByPath(simplifiedGraph);
    const bundleReferences = result.getBundleReferences();
    const indexKey = contentKeysByPath.get('index.js') ?? -1;
    const childKey = contentKeysByPath.get('child.png') ?? -1;
    assert.deepEqual(
      bundleReferences.get(indexKey)?.map((r) => r.assetContentKey) ?? [],
      [childKey],
    );
    assert.deepEqual(
      bundleReferences.get(childKey)?.map((r) => r.assetContentKey) ?? [],
      [],
    );
  });
});
