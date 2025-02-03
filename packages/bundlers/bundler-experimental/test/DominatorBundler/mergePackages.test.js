// @flow strict-local

import path from 'path';
import {fsFixture, overlayFS, workerFarm} from '@atlaspack/test-utils';
import assert from 'assert';
import {mergedDominatorsToDot} from '../graphviz/GraphvizUtils';
import {asset, fixtureFromGraph} from '../fixtureFromGraph';
import {dotTest, setupBundlerTest, testMakePackageKey} from '../test-utils';
import {findAssetDominators} from '../../src/DominatorBundler/findAssetDominators';
import {bundleGraphToRootedGraph} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import {createPackages} from '../../src/DominatorBundler/createPackages';
import {
  buildPackageGraph,
  runMergePackages,
  getPackageInformation,
} from '../../src/DominatorBundler/mergePackages';

describe('mergePackages', () => {
  const fixture1 = async () => {
    const entryDir = path.join(__dirname, 'test');
    const entryPath1 = path.join(entryDir, 'page1.js');
    const entryPath2 = path.join(entryDir, 'page2.js');
    await fixtureFromGraph(entryDir, overlayFS, [
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
    return {mutableBundleGraph, entryDir};
  };

  describe('buildPackageGraph', () => {
    before(async function () {
      this.timeout(10000);
      // Warm up worker farm so that the first test doesn't account for this time.
      await workerFarm.callAllWorkers('ping', []);
    });

    dotTest(
      __filename,
      'creates the relationship between packages based on the relationship between assets',
      async () => {
        const {mutableBundleGraph, entryDir} = await fixture1();
        const {dominators} = findAssetDominators(mutableBundleGraph);
        const rootedGraph =
          bundleGraphToRootedGraph(mutableBundleGraph).getGraph();
        const packages = createPackages(
          rootedGraph,
          dominators,
          (parentChunks) =>
            testMakePackageKey(entryDir, dominators, parentChunks),
        );

        const packageNodes = packages.getNodeIdsConnectedFrom(
          packages.getNodeIdByContentKey('root'),
        );
        const packageInfos = packageNodes.map((nodeId) => {
          const packageNode = packages.getNode(nodeId);
          if (packageNode == null || packageNode === 'root') {
            return null;
          }
          return getPackageInformation(packages, nodeId, packageNode);
        });

        const packageGraph = buildPackageGraph(
          rootedGraph,
          packages,
          packageNodes,
          packageInfos,
        );
        const dot = mergedDominatorsToDot(entryDir, packageGraph);

        assert.equal(
          dot,
          `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "package:page1.js,page2.js";
  "page1.js";
  "page2.js";
  "root";

  "page1.js" -> "package:page1.js,page2.js";
  "page2.js" -> "package:page1.js,page2.js";
  "root" -> "package:page1.js,page2.js";
  "root" -> "page1.js";
  "root" -> "page2.js";
}
      `.trim(),
        );

        return [
          {
            label: 'package-graph',
            dot,
          },
        ];
      },
    );

    // we need to connect async dependencies to their parents on the simple graph
    // but keep a separate edge type perhaps
    dotTest(
      __filename,
      'correctly creates simple async asset relationships',
      async () => {
        await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        async.js:
          module.exports = () => 1234;
        index.js:
          output(import('./async').then((get) => {
            return get();
          }));
    `;
        const entryDir = path.join(__dirname, 'bundler-experimental');
        const {mutableBundleGraph} = await setupBundlerTest(
          path.join(entryDir, 'index.js'),
        );

        const {dominators} = findAssetDominators(mutableBundleGraph);
        const rootedGraph =
          bundleGraphToRootedGraph(mutableBundleGraph).getGraph();
        const packages = createPackages(
          rootedGraph,
          dominators,
          (parentChunks) =>
            testMakePackageKey(entryDir, dominators, parentChunks),
        );

        const packageNodes = packages.getNodeIdsConnectedFrom(
          packages.getNodeIdByContentKey('root'),
        );
        const packageInfos = packageNodes.map((nodeId) => {
          const packageNode = packages.getNode(nodeId);
          if (packageNode == null || packageNode === 'root') {
            return null;
          }
          return getPackageInformation(packages, nodeId, packageNode);
        });

        const packageGraph = buildPackageGraph(
          rootedGraph,
          packages,
          packageNodes,
          packageInfos,
        );
        const dot = mergedDominatorsToDot(entryDir, packageGraph);

        assert.equal(
          dot,
          `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "async.js";
  "index.js";
  "root";

  "index.js" -> "async.js";
  "root" -> "async.js";
  "root" -> "index.js";
}
      `.trim(),
        );

        return [
          {
            label: 'package-graph',
            dot,
          },
        ];
      },
    );

    dotTest(
      __filename,
      'correctly creates async package asset relationships',
      async () => {
        await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        lib.js:
          module.exports = () => 1;
        async.js:
          const r = require('./lib');
          module.exports = () => 32 + r();
        dependency.js:
          const r = require('./lib');
          module.exports = () => 1200 + r();
        index.js:
          const get = require('./dependency');
          output(import('./async').then((get2) => {
            return get() + get2();
          }));
    `;
        const entryDir = path.join(__dirname, 'bundler-experimental');
        const {mutableBundleGraph} = await setupBundlerTest(
          path.join(entryDir, 'index.js'),
        );

        const {dominators} = findAssetDominators(mutableBundleGraph);
        const rootedGraph =
          bundleGraphToRootedGraph(mutableBundleGraph).getGraph();
        const packages = createPackages(
          rootedGraph,
          dominators,
          (parentChunks) =>
            testMakePackageKey(entryDir, dominators, parentChunks),
        );

        const packageNodes = packages.getNodeIdsConnectedFrom(
          packages.getNodeIdByContentKey('root'),
        );
        const packageInfos = packageNodes.map((nodeId) => {
          const packageNode = packages.getNode(nodeId);
          if (packageNode == null || packageNode === 'root') {
            return null;
          }
          return getPackageInformation(packages, nodeId, packageNode);
        });

        const packageGraph = buildPackageGraph(
          rootedGraph,
          packages,
          packageNodes,
          packageInfos,
        );
        const dot = mergedDominatorsToDot(entryDir, packageGraph);

        assert.equal(
          dot,
          `
digraph merged {
  labelloc="t";
  label="Merged";
  layout="dot";

  "async.js";
  "index.js";
  "package:async.js,index.js";
  "root";

  "async.js" -> "package:async.js,index.js";
  "index.js" -> "async.js";
  "index.js" -> "package:async.js,index.js";
  "root" -> "async.js";
  "root" -> "index.js";
  "root" -> "package:async.js,index.js";
}
      `.trim(),
        );

        return [
          {
            label: 'package-graph',
            dot,
          },
        ];
      },
    );
  });

  describe('mergePackages', () => {
    dotTest(
      __filename,
      'merges packages onto parents based on size',
      async () => {
        const {mutableBundleGraph, entryDir} = await fixture1();
        const {dominators} = findAssetDominators(mutableBundleGraph);
        const rootedGraph =
          bundleGraphToRootedGraph(mutableBundleGraph).getGraph();
        const packages = createPackages(rootedGraph, dominators);
        const result = runMergePackages(rootedGraph, packages);
        const dot = mergedDominatorsToDot(entryDir, result, 'Duplicated');

        assert.equal(
          dot,
          `
digraph merged {
  labelloc="t";
  label="Duplicated";
  layout="dot";

  "jsx.js";
  "left-pad.js";
  "lodash.js";
  "page1.js";
  "page2.js";
  "react.js";
  "root";
  "string-chart-at.js";
  "string-concat.js";

  "page1.js" -> "left-pad.js";
  "page1.js" -> "lodash.js";
  "page1.js" -> "react.js";
  "page1.js" -> "string-concat.js";
  "page2.js" -> "left-pad.js";
  "page2.js" -> "lodash.js";
  "page2.js" -> "react.js";
  "page2.js" -> "string-concat.js";
  "react.js" -> "jsx.js";
  "root" -> "page1.js";
  "root" -> "page2.js";
  "string-concat.js" -> "string-chart-at.js";
}
      `.trim(),
        );

        return [
          {
            label: 'package-graph',
            dot,
          },
        ];
      },
    );
  });
});
