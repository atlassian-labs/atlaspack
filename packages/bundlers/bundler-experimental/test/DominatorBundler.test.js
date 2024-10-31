// @flow strict-local

import * as path from 'path';
import type {Asset} from '@atlaspack/types';
import {overlayFS} from '@atlaspack/test-utils';
import {setupBundlerTest} from './test-utils';
import {
  bundleGraphToRootedGraph,
  createPackages,
  findAssetDominators,
} from '../src/DominatorBundler';
import assert from 'assert';
import {asset, fixtureFromGraph} from './fixture-from-dot';
import {execSync} from 'child_process';
import fs from 'fs';
import {mkdirSync} from 'fs';
import nullthrows from 'nullthrows';

function dominatorsToDot(
  entryPath: string,
  dominators: Map<Asset, Set<Asset>>,
): string {
  const contents = [];
  const cleanPath = (p) => {
    if (p.includes('esmodule-helpers.js')) {
      return 'esmodule_helpers.js';
    }
    return path.relative(entryPath, p);
  };

  const iterableDominators = Array.from(dominators.entries());
  iterableDominators.sort((a, b) =>
    cleanPath(a[0].filePath).localeCompare(cleanPath(b[0].filePath)),
  );

  for (let [asset] of iterableDominators) {
    const assetPath = cleanPath(asset.filePath);
    contents.push(`"${assetPath}";`);
  }

  contents.push('');

  for (let [asset, dominatorSet] of iterableDominators) {
    const assetPath = cleanPath(asset.filePath);
    const iterableDominatorSet = Array.from(dominatorSet).sort((a, b) =>
      cleanPath(a.filePath).localeCompare(cleanPath(b.filePath)),
    );

    for (let dominated of iterableDominatorSet) {
      if (dominated === asset) {
        continue;
      }

      const dominatedPath = cleanPath(dominated.filePath);
      contents.push(`"${dominatedPath}" -> "${assetPath}";`);
    }
  }

  return `
digraph dominators {
  labelloc="t";
  label="Dominators";

${contents.map((l) => (l.length > 0 ? `  ${l}` : '')).join('\n')}
}`.trim();
}

function mergedDominatorsToDot(
  entryPath: string,
  dominators: Map<string, Set<Asset>>,
): string {
  const contents = [];
  const cleanPath = (p) => {
    if (p.includes('esmodule-helpers.js')) {
      return 'esmodule_helpers.js';
    }
    return path.relative(entryPath, p);
  };

  const iterableDominators = Array.from(dominators.entries());
  iterableDominators.sort((a, b) =>
    cleanPath(a[0]).localeCompare(cleanPath(b[0])),
  );

  contents.push('');

  for (let [asset, dominatorSet] of iterableDominators) {
    const clusterId = asset === '' ? 'empty' : asset.replaceAll(',', '__');
    contents.push(`subgraph cluster_${clusterId} {
  label="";
    `);
    const iterableDominatorSet = Array.from(dominatorSet).sort((a, b) =>
      cleanPath(a.filePath).localeCompare(cleanPath(b.filePath)),
    );

    for (let dominated of iterableDominatorSet) {
      if (dominated === asset) {
        continue;
      }

      const dominatedPath = cleanPath(dominated.filePath);
      contents.push(`"${dominatedPath}";`);
    }

    contents.push('}');
  }

  return `
digraph dominators {
  labelloc="t";
  label="Dominators";

${contents.map((l) => (l.length > 0 ? `  ${l}` : '')).join('\n')}
}`.trim();
}

function runDotForTest(name: string, label: string, dot: string) {
  const slugTestName = path.join(
    __dirname,
    '__graphs__',
    path.basename(__filename) + ' - ' + name,
  );
  mkdirSync(slugTestName, {recursive: true});
  mkdirSync(path.join(slugTestName, 'dot'), {recursive: true});
  mkdirSync(path.join(slugTestName, 'png'), {recursive: true});
  const filePath = `${label}.dot`;
  fs.writeFileSync(path.join(slugTestName, 'dot', filePath), dot);
  execSync(
    `dot -Tpng -o "${path.join(
      slugTestName,
      'png',
      filePath,
    )}.png" "${path.join(slugTestName, 'dot', filePath)}"`,
  );
}

describe.only('DominatorBundler', () => {
  function test(
    name: string,
    fn: () => Promise<{|label: string, dot: string|}[]>,
  ) {
    it(name, async () => {
      const graphs = await fn();

      graphs.forEach(({label, dot}) => {
        runDotForTest(name, label, dot);
      });
    });
  }

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
    test('can find dominators for a simple graph', async () => {
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

    test('can find dominators for a slightly more complex graph', async () => {
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

  "esmodule_helpers.js";
  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "page.js";
  "react.js";
  "string-chart-at.js";
  "string-concat.js";

  "page.js" -> "esmodule_helpers.js";
  "react.js" -> "jsx.js";
  "page.js" -> "left-pad.js";
  "page.js" -> "lodash.js";
  "page.js" -> "react.js";
  "string-concat.js" -> "string-chart-at.js";
  "page.js" -> "string-concat.js";
}
            `.trim(),
      );

      return [
        {label: 'input', dot: inputDot},
        {label: 'output', dot: outputDot},
      ];
    });

    test('works when there are multiple entry-points', async () => {
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

      const mergedDominators = createPackages(mutableBundleGraph, dominators);

      return [
        {label: 'input', dot: inputDot},
        {label: 'output', dot: outputDot},
        {
          label: 'merged',
          dot: mergedDominatorsToDot(entryDir, mergedDominators),
        },
      ];
    });
  });
});
