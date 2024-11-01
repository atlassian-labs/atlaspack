// @flow strict-local

import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import {rootedGraphToDot} from '../graphviz/GraphvizUtils';
import nullthrows from 'nullthrows';
import {dotTest, setupBundlerTest} from '../test-utils';
import * as path from 'path';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import assert from 'assert';

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

    assert.equal(rootGraph.nodes.length, 4);

    const rootNode = rootGraph.getNodeIdByContentKey('root');
    const assetIdsByPath = new Map();
    rootGraph.traverse((node) => {
      if (node !== rootNode) {
        const asset = rootGraph.getNode(node);
        if (!asset || typeof asset === 'string') {
          throw new Error('Asset not found');
        }
        assetIdsByPath.set(path.basename(asset.filePath), asset.id);
      }
    }, rootNode);

    const getConnections = (contentKey: string) => {
      const node = rootGraph.getNodeIdByContentKey(contentKey);
      return rootGraph.getNodeIdsConnectedFrom(node).map((nodeId) => {
        const node = rootGraph.getNode(nodeId);
        if (!node || typeof node === 'string') throw new Error('root cycle');
        return path.basename(node.filePath);
      });
    };

    assert.deepEqual(getConnections('root'), ['test.js']);
    assert.deepEqual(
      getConnections(nullthrows(assetIdsByPath.get('test.js'))),
      ['dependency.js', 'esmodule-helpers.js'],
    );
    assert.deepEqual(
      getConnections(nullthrows(assetIdsByPath.get('dependency.js'))),
      ['esmodule-helpers.js'],
    );
    assert.deepEqual(
      getConnections(nullthrows(assetIdsByPath.get('esmodule-helpers.js'))),
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

      const simplfiedGraph = bundleGraphToRootedGraph(mutableBundleGraph);
      const dot = rootedGraphToDot(
        entryDir,
        simplfiedGraph,
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
  "esmodule_helpers.js";
  "library1.js";
  "library2.js";
  "library3.js";
  "page1.js";
  "page2.js";

  "library1.js" -> "esmodule_helpers.js";
  "library1.js" -> "library3.js";
  "library2.js" -> "esmodule_helpers.js";
  "library2.js" -> "library3.js";
  "library3.js" -> "esmodule_helpers.js";
  "page1.js" -> "esmodule_helpers.js";
  "page1.js" -> "library1.js";
  "page2.js" -> "esmodule_helpers.js";
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
});
