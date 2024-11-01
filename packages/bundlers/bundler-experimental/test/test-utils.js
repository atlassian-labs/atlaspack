// @flow strict-local

import BundleGraph from '@atlaspack/core/src/BundleGraph';
import MutableBundleGraph from '@atlaspack/core/src/public/MutableBundleGraph';
import Atlaspack from '@atlaspack/core';
import {getPublicDependency} from '@atlaspack/core/src/public/Dependency';
import AssetGraph from '@atlaspack/core/src/AssetGraph';
import {assetFromValue} from '@atlaspack/core/src/public/Asset';
import resolveOptions from '@atlaspack/core/src/resolveOptions';
import {getParcelOptions, overlayFS} from '@atlaspack/test-utils';
import type {Asset, Dependency} from '@atlaspack/types';
import * as path from 'path';
import makeDebug from 'debug';
import {runDotForTest} from './graphviz/GraphvizUtils';

const debug = makeDebug('atlaspack:bundler:working-bundler:test-utils');

export function dotTest(
  __filename: string,
  name: string,
  fn: () => Promise<{|label: string, dot: string|}[]>,
) {
  it(name, async () => {
    const graphs = await fn();

    graphs.forEach(({label, dot}) => {
      runDotForTest(path.dirname(__filename), __filename, name, label, dot);
    });
  });
}

export interface BundlerTestSetup {
  assetGraph: AssetGraph;
  mutableBundleGraph: MutableBundleGraph;
  bundleGraph: BundleGraph;
  entries: {|
    entryAsset: Asset,
    entryDependency: Dependency,
  |}[];
}

/**
 * Given an entry path, creates the asset graph so we can run bundlers over it.
 */
export async function setupBundlerTest(
  entryPath: string | string[],
): Promise<BundlerTestSetup> {
  const options = getParcelOptions(entryPath, {
    inputFS: overlayFS,
    defaultConfig: path.join(__dirname, 'atlaspack-config.json'),
  });
  debug('Resolving options', entryPath);
  const resolvedOptions = await resolveOptions(options);
  debug('Creating atlaspack instance and clearing caches');
  const atlaspack = new Atlaspack(options);

  // We must clear caches
  await atlaspack.clearBuildCaches();
  // For some reason atlaspack doesn't have the proper ref yet when initialized
  // so we manually clear the worker build caches between tests too.
  //
  // Since all workers are shared between tests, and due to how atlaspack is
  // written, we must clear caches otherwise we'll get weird errors.
  await options.workerFarm?.callAllWorkers('clearWorkerBuildCaches', []);

  debug('Building asset graph');
  const {assetGraph} = await atlaspack.unstable_buildAssetGraph();

  debug('Building bundle graph and finding entry values for bundling');
  const bundleGraph = BundleGraph.fromAssetGraph(assetGraph, false);

  const entryPaths = Array.isArray(entryPath) ? entryPath : [entryPath];
  const entries = entryPaths.map((entryPath) => {
    const entryAssetValue = assetGraph
      .getEntryAssets()
      .find(
        (entryAssetValue) =>
          entryAssetValue.filePath ===
          path.relative(resolvedOptions.projectRoot, entryPath),
      );
    if (!entryAssetValue) {
      throw new Error('Entry asset not found');
    }

    const entryAsset: Asset = assetFromValue(entryAssetValue, resolvedOptions);
    const entryDependencyValue = assetGraph
      .getIncomingDependencies(entryAssetValue)
      .find((dependency) => dependency.isEntry);
    if (!entryDependencyValue) {
      throw new Error('Entry dependency not found');
    }
    const entryDependency = getPublicDependency(
      entryDependencyValue,
      resolvedOptions,
    );

    return {entryAsset, entryDependency};
  });

  const mutableBundleGraph = new MutableBundleGraph(
    bundleGraph,
    resolvedOptions,
  );

  debug('Set-up finished');
  return {
    atlaspack,
    assetGraph,
    mutableBundleGraph,
    bundleGraph,
    entries,
  };
}
