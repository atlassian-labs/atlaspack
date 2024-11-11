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
import type {AssetGraphNode, AssetGroup} from '../../../core/core/src/types';

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

function getAssets(assetGraph: AssetGraph): Array<AssetGraphNode> {
  const assets = [];
  assetGraph.traverse((nodeId) => {
    const node = assetGraph.getNode(nodeId);
    if (node?.type === 'asset' || node?.type === 'asset_group') {
      assets.push(node);
    }
  });
  return assets;
}

function getBundleAssets(bundleGraph: BundleGraph): Array<Asset> {
  const assets = [];
  bundleGraph.traverse((node) => {
    if (node.type === 'asset') {
      assets.push(node.value);
    }
  });
  return assets;
}

function simplifyAsset(node: AssetGraphNode) {
  if (node.type !== 'asset') {
    return node;
  }
  const asset = node.value;

  const dependencies = new Map();
  const simplifySymbol = (symbol) => ({
    ...symbol,
    local: null,
    id: null,
  });
  const simplifySymbols = (symbols) => {
    if (symbols == null) return null;
    const result = new Map();
    for (const symbolKey of symbols?.keys() ?? []) {
      const symbolValue = symbols?.get(symbolKey);
      result.set(symbolKey, simplifySymbol(symbolValue));
    }
    return result;
  };
  for (const dependencyId of asset.dependencies.keys()) {
    const dependency = asset.dependencies.get(dependencyId);
    if (!dependency) continue;
    const simpleId =
      String(dependency.sourcePath) + '://' + dependency.specifier;
    let symbols = simplifySymbols(dependency.symbols);
    dependencies.set(simpleId, {
      ...dependency,
      id: simpleId,
      sourceAssetId: null,
      symbols,
      env: {
        ...dependency.env,
        id: null,
      },
    });
  }

  return {
    ...asset,
    id: asset.filePath,
    dependencies,
    symbols: simplifySymbols(asset.symbols),
    meta: {
      ...asset.meta,
      id: null,
    },
    env: {
      ...asset.env,
      id: null,
    },
    contentKey: null,
    uniqueKey: null,
    stats: null,
    outputHash: null,
    mapKey: null,
  };
}

function diffCaches(cache: Cache, diffCache: Cache) {
  {
    // Diff all assets
    const cacheAssets = (cache.assetGraph && getAssets(cache.assetGraph)) ?? [];
    const diffAssets =
      (diffCache.assetGraph && getAssets(diffCache.assetGraph)) ?? [];
    // const cacheAssets =
    //   (cache.bundleGraph && getBundleAssets(cache.bundleGraph)) ?? [];
    // const diffAssets =
    //   (diffCache.bundleGraph && getBundleAssets(diffCache.bundleGraph)) ?? [];

    console.log('Found assets in cache:', cacheAssets.length);
    console.log('Found assets in diff:', diffAssets.length);

    const cacheAssetsByFilePath = groupBy(
      cacheAssets,
      (asset) => asset.filePath,
    );
    const diffAssetsByFilePath = groupBy(diffAssets, (asset) => asset.filePath);

    Object.keys(cacheAssetsByFilePath).forEach((filePath) => {
      // if (!filePath.includes('PrivacySecurity.js')) return;
      const cacheAssets = cacheAssetsByFilePath[filePath] ?? [];
      const diffAssets = diffAssetsByFilePath[filePath] ?? [];
      cacheAssets.sort((a, b) => a.type.localeCompare(b.type));
      diffAssets.sort((a, b) => a.type.localeCompare(b.type));

      const simplifiedCachedAssets = cacheAssets.map((asset) =>
        simplifyAsset(asset),
      );
      const simplifiedDiffAssets = diffAssets.map((asset) =>
        simplifyAsset(asset),
      );
      if (!isEqual(simplifiedCachedAssets, simplifiedDiffAssets)) {
        console.log(filePath);
        console.log(diff(simplifiedCachedAssets, simplifiedDiffAssets));
      }
    });
  }
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

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
