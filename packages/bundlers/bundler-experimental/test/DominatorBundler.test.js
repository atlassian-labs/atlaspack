// @flow strict-local

import * as path from 'path';
import type {Asset} from '@atlaspack/types';
import {overlayFS} from '@atlaspack/test-utils';
import {setupBundlerTest} from './test-utils';
import {
  findAssetDominators,
  getImmediateDominatorTree,
} from '../src/DominatorBundler';
import assert from 'assert';
import {asset, fixtureFromGraph} from './fixture-from-dot';
import {execSync} from 'child_process';
import fs from 'fs';
import {mkdirSync} from 'fs';

function dominatorsToDot(
  entryPath: string,
  dominators: Map<Asset, Set<Asset>>,
): string {
  const contents = [];
  const cleanPath = (p) => {
    if (p.includes('esmodule-helpers.js')) {
      return 'esmodule_helpers.js';
    }
    return path.relative(path.dirname(entryPath), p);
  };

  for (let [asset] of dominators) {
    const assetPath = cleanPath(asset.filePath);
    contents.push(`"${assetPath}";`);
  }

  contents.push('');

  for (let [asset, dominatorSet] of dominators) {
    const assetPath = cleanPath(asset.filePath);
    for (let dominated of dominatorSet) {
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

  describe('findAssetDominators', () => {
    test('can find dominators for a simple graph', async () => {
      const entryPath = path.join(__dirname, 'test/test.js');
      const inputDot = await fixtureFromGraph(
        path.dirname(entryPath),
        overlayFS,
        [
          asset('test.js', ['dependency.js', {to: 'async.js', type: 'async'}]),
          asset('async.js', []),
          asset('dependency.js', []),
        ],
      );

      const {mutableBundleGraph, entry} = await setupBundlerTest(entryPath);
      const dominators = findAssetDominators(mutableBundleGraph, [
        entry.entryAsset,
      ]);

      const outputDot = dominatorsToDot(entryPath, dominators);
      assert.equal(
        outputDot,
        `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "test.js";
  "dependency.js";
  "esmodule_helpers.js";
  "async.js";

  "test.js" -> "dependency.js";
  "test.js" -> "esmodule_helpers.js";
  "test.js" -> "async.js";
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
      const inputDot = await fixtureFromGraph(
        path.dirname(entryPath),
        overlayFS,
        [
          asset('page.js', ['react.js', 'lodash.js']),
          asset('react.js', ['left-pad.js', 'string-concat.js', 'jsx.js']),
          asset('lodash.js', ['left-pad.js']),
          asset('left-pad.js', ['string-concat.js']),
          asset('jsx.js', []),
          asset('string-concat.js', ['string-chart-at.js']),
          asset('string-chart-at.js', []),
        ],
      );

      const {mutableBundleGraph, entry} = await setupBundlerTest(entryPath);
      const dominators = findAssetDominators(mutableBundleGraph, [
        entry.entryAsset,
      ]);

      const outputDot = dominatorsToDot(entryPath, dominators);
      assert.equal(
        outputDot,
        `
digraph dominators {
  labelloc="t";
  label="Dominators";

  "page.js";
  "react.js";
  "left-pad.js";
  "string-concat.js";
  "string-chart-at.js";
  "esmodule_helpers.js";
  "jsx.js";
  "lodash.js";

  "page.js" -> "react.js";
  "page.js" -> "left-pad.js";
  "page.js" -> "string-concat.js";
  "string-concat.js" -> "string-chart-at.js";
  "page.js" -> "string-chart-at.js";
  "page.js" -> "esmodule_helpers.js";
  "react.js" -> "jsx.js";
  "page.js" -> "jsx.js";
  "page.js" -> "lodash.js";
}
            `.trim(),
      );

      return [
        {label: 'input', dot: inputDot},
        {label: 'output', dot: outputDot},
      ];
    });
  });
});
