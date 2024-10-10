// @flow strict-local
/* eslint-disable no-console, monorepo/no-internal-import */

import commander from 'commander';
// $FlowFixMe
import {diff} from 'jest-diff';
import {loadGraphs} from './index.js';
import type RequestTracker from '@atlaspack/core/src/RequestTracker';
import type AssetGraph from '@atlaspack/core/src/AssetGraph';
import type BundleGraph from '@atlaspack/core/src/BundleGraph';
import type {Asset} from '@atlaspack/core/src/types';

// $FlowFixMe
import groupBy from 'lodash/groupBy';
// $FlowFixMe
import isEqual from 'lodash/isEqual';

type Cache = {
  assetGraph: ?AssetGraph,
  bundleGraph: ?BundleGraph,
  requestTracker: ?RequestTracker,
  ...
};

type Options = {|
  cacheDir: string,
  diffDir: string,
|};

function getAssets(assetGraph: AssetGraph): Array<Asset> {
  const assets = [];
  assetGraph.traverse(nodeId => {
    const node = assetGraph.getNode(nodeId);
    if (node?.type === 'asset') {
      assets.push(node.value);
    }
  });
  return assets;
}

function getBundleAssets(bundleGraph: BundleGraph): Array<Asset> {
  const assets = [];
  bundleGraph.traverse(node => {
    if (node.type === 'asset') {
      assets.push(node.value);
    }
  });
  return assets;
}

function simplifyAsset(asset: Asset) {
  const dependencies = new Map();
  for (const dependencyId of asset.dependencies.keys()) {
    const dependency = asset.dependencies.get(dependencyId);
    if (!dependency) continue;
    const simpleId =
      String(dependency.sourcePath) + '://' + dependency.specifier;
    dependencies.set(simpleId, {
      id: simpleId,
      ...dependency,
    });
  }

  return {
    id: asset.filePath,
    contentKey: null,
    stats: null,
    outputHash: null,
    mapKey: null,
    ...asset,
    dependencies,
  };
}

function diffCaches(cache: Cache, diffCache: Cache) {
  // Diff all assets
  // const cacheAssets = (cache.assetGraph && getAssets(cache.assetGraph)) ?? [];
  // const diffAssets =
  //   (diffCache.assetGraph && getAssets(diffCache.assetGraph)) ?? [];
  const cacheAssets =
    (cache.bundleGraph && getBundleAssets(cache.bundleGraph)) ?? [];
  const diffAssets =
    (diffCache.bundleGraph && getBundleAssets(diffCache.bundleGraph)) ?? [];

  console.log('Found assets in cache:', cacheAssets.length);
  console.log('Found assets in diff:', diffAssets.length);

  const cacheAssetsByFilePath = groupBy(cacheAssets, asset => asset.filePath);
  const diffAssetsByFilePath = groupBy(diffAssets, asset => asset.filePath);

  Object.keys(cacheAssetsByFilePath).forEach(filePath => {
    if (!filePath.includes('PrivacySecurity.js')) return;
    const cacheAssets = cacheAssetsByFilePath[filePath] ?? [];
    const diffAssets = diffAssetsByFilePath[filePath] ?? [];
    cacheAssets.sort((a, b) => a.id.localeCompare(b.id));
    diffAssets.sort((a, b) => a.id.localeCompare(b.id));

    const simplifiedCachedAssets = cacheAssets.map(asset =>
      simplifyAsset(asset),
    );
    const simplifiedDiffAssets = diffAssets.map(asset => simplifyAsset(asset));
    if (!isEqual(simplifiedCachedAssets, simplifiedDiffAssets)) {
      console.log(filePath);
      console.log(diff(simplifiedCachedAssets, simplifiedDiffAssets));
    }
  });
}

async function main() {
  console.log('cache-diff');

  const program = new commander.Command();
  program.option('--cache-dir <dir>', 'The cache directory to diff');
  program.option('--diff-dir <dir>', 'The directory to diff the cache against');
  program.parse(process.argv);

  // $FlowFixMe
  const options: Options = program.opts();
  const cacheDir = String(options.cacheDir);
  const diffDir = String(options.diffDir);

  console.log('Loading cache directory', cacheDir);
  const cache = await loadGraphs(cacheDir);
  console.log('Loading cache directory', diffDir);
  const diff = await loadGraphs(diffDir);

  diffCaches(cache, diff);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
