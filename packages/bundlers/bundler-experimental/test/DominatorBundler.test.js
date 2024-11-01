// @flow strict-local

import * as path from 'path';
import {overlayFS} from '@atlaspack/test-utils';
import {
  dominatorsToDot,
  dotTest,
  mergedDominatorsToDot,
  setupBundlerTest,
} from './test-utils';
import {
  bundleGraphToRootedGraph,
  createPackages,
  findAssetDominators,
} from '../src/DominatorBundler';
import assert from 'assert';
import {asset, fixtureFromGraph} from './fixture-from-dot';
import nullthrows from 'nullthrows';

describe.only('DominatorBundler', () => {
  describe('bundleGraphToRootedGraph', () => {
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
  });

  describe('findAssetDominators', () => {
    dotTest(__filename, 'can find dominators for a simple graph', async () => {
      const entryPath = path.join(__dirname, 'test/test.js');
      const entryDir = path.dirname(entryPath);
      const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
        asset('test.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
        asset('async.js', []),
        asset('dependency.js', []),
      ]);

      const {mutableBundleGraph} = await setupBundlerTest(entryPath);
      const dominators = findAssetDominators(mutableBundleGraph);

      const outputDot = dominatorsToDot(entryDir, dominators);
      assert.equal(
        outputDot,
        `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "root";
  "root" -> "test.js";
  "async.js";
  "dependency.js";
  "esmodule_helpers.js";
  "test.js";

  "test.js" -> "async.js";
  "test.js" -> "dependency.js";
  "test.js" -> "esmodule_helpers.js";
}
      `.trim(),
      );

      return [
        {label: 'input', dot: inputDot},
        {label: 'output', dot: outputDot},
      ];
    });

    dotTest(
      __filename,
      'can find dominators for a slightly more complex graph',
      async () => {
        const entryPath = path.join(__dirname, 'test/page.js');
        const entryDir = path.dirname(entryPath);
        const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
          asset('page.js', ['react.js', 'lodash.js']),
          asset('react.js', ['left-pad.js', 'string-concat.js', 'jsx.js']),
          asset('lodash.js', ['left-pad.js']),
          asset('left-pad.js', ['string-concat.js']),
          asset('jsx.js', []),
          asset('string-concat.js', ['string-chart-at.js']),
          asset('string-chart-at.js', []),
        ]);

        const {mutableBundleGraph} = await setupBundlerTest(entryPath);
        const dominators = findAssetDominators(mutableBundleGraph);

        const outputDot = dominatorsToDot(entryDir, dominators);
        assert.equal(
          outputDot,
          `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "root";
  "root" -> "page.js";
  "esmodule_helpers.js";
  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "page.js";
  "react.js";
  "string-chart-at.js";
  "string-concat.js";

  "page.js" -> "esmodule_helpers.js";
  "page.js" -> "left-pad.js";
  "page.js" -> "lodash.js";
  "page.js" -> "react.js";
  "page.js" -> "string-concat.js";
  "react.js" -> "jsx.js";
  "string-concat.js" -> "string-chart-at.js";
}
            `.trim(),
        );

        return [
          {label: 'input', dot: inputDot},
          {label: 'output', dot: outputDot},
        ];
      },
    );

    dotTest(
      __filename,
      'works when there are multiple entry-points',
      async () => {
        const entryDir = path.join(__dirname, 'test');
        const entryPath1 = path.join(entryDir, 'page1.js');
        const entryPath2 = path.join(entryDir, 'page2.js');
        const inputDot = await fixtureFromGraph(entryDir, overlayFS, [
          asset('page1.js', ['react.js', 'lodash.js']),
          asset('page2.js', ['lodash.js', 'react.js']),
          asset('react.js', ['left-pad.js', 'string-concat.js', 'jsx.js']),
          asset('lodash.js', ['left-pad.js']),
          asset('left-pad.js', ['string-concat.js']),
          asset('jsx.js', []),
          asset('string-concat.js', ['string-chart-at.js']),
          asset('string-chart-at.js', []),
        ]);

        const {mutableBundleGraph} = await setupBundlerTest([
          entryPath1,
          entryPath2,
        ]);
        const dominators = findAssetDominators(mutableBundleGraph);

        const outputDot = dominatorsToDot(entryDir, dominators);
        assert.equal(
          outputDot,
          `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "root";
  "root" -> "esmodule_helpers.js";
  "root" -> "left-pad.js";
  "root" -> "lodash.js";
  "root" -> "page1.js";
  "root" -> "page2.js";
  "root" -> "react.js";
  "root" -> "string-concat.js";
  "esmodule_helpers.js";
  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "page1.js";
  "page2.js";
  "react.js";
  "string-chart-at.js";
  "string-concat.js";

  "react.js" -> "jsx.js";
  "string-concat.js" -> "string-chart-at.js";
}
            `.trim(),
        );

        const iterations = [];
        const mergedDominators = createPackages(
          mutableBundleGraph,
          dominators,
          (graph, label) => {
            iterations.push({
              label: `merging iteration ${label}`,
              dot: mergedDominatorsToDot(entryDir, graph, label),
            });
          },
        );
        const mergedDominatorsDot = mergedDominatorsToDot(
          entryDir,
          mergedDominators,
        );

        assert.equal(
          mergedDominatorsDot,
          `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "root";
  "page2.js";
  "page1.js";
  "package_1,9";
  "lodash.js";
  "react.js";
  "jsx.js";
  "left-pad.js";
  "string-concat.js";
  "string-chart-at.js";
  "package_1,10,9";
  "esmodule_helpers.js";

  "root" -> "page2.js";
  "root" -> "page1.js";
  "root" -> "package_1,9";
  "root" -> "package_1,10,9";
  "package_1,9" -> "lodash.js";
  "package_1,9" -> "react.js";
  "package_1,9" -> "left-pad.js";
  "package_1,9" -> "string-concat.js";
  "react.js" -> "jsx.js";
  "string-concat.js" -> "string-chart-at.js";
  "package_1,10,9" -> "esmodule_helpers.js";
}
          `.trim(),
        );

        return [
          {label: 'input', dot: inputDot},
          {label: 'output', dot: outputDot},
          {
            label: 'merged',
            dot: mergedDominatorsDot,
          },
          ...iterations,
        ];
      },
    );
  });
});
