// @flow strict-local

import * as path from 'path';
import {overlayFS, workerFarm} from '@atlaspack/test-utils';
import {dotTest, setupBundlerTest} from './test-utils';
import {createPackages} from '../src/DominatorBundler';
import assert from 'assert';
import {asset, fixtureFromGraph} from './fixtureFromGraph';
import {
  rootedGraphToDot,
  mergedDominatorsToDot,
} from './graphviz/GraphvizUtils';
import {findAssetDominators} from '../src/DominatorBundler/findAssetDominators';

describe('DominatorBundler', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  describe('createPackages - all together now', () => {
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

        const outputDot = rootedGraphToDot(entryDir, dominators);

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
