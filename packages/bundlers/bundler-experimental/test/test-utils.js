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
import * as fs from 'fs';
import {mkdirSync} from 'fs';
import {execSync} from 'child_process';
import {ContentGraph} from '@atlaspack/graph';
import type {PackagedDominatorGraph} from '../src/DominatorBundler';
import nullthrows from 'nullthrows';

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

export function dominatorsToDot(
  entryDir: string,
  dominators: ContentGraph<Asset | 'root'>,
): string {
  const contents = [];
  const cleanPath = (p) => {
    if (p.includes('esmodule-helpers.js')) {
      return 'esmodule_helpers.js';
    }
    return path.relative(entryDir, p);
  };

  contents.push('"root";');
  const rootNodeId = dominators.getNodeIdByContentKey('root');
  const rootNodes = dominators
    .getNodeIdsConnectedFrom(rootNodeId)
    .map((id) => {
      const node = dominators.getNode(id);
      if (node && node !== 'root') {
        return cleanPath(node.filePath);
      }
    })
    .filter(Boolean)
    .sort();
  rootNodes.forEach((node) => {
    contents.push(`"root" -> "${node}";`);
  });

  const iterableDominators: Asset[] = [];
  dominators.nodes.forEach((node) => {
    if (node && node !== 'root') {
      iterableDominators.push(node);
    }
  });
  iterableDominators.sort((a, b) =>
    cleanPath(a.filePath).localeCompare(cleanPath(b.filePath)),
  );

  for (let asset of iterableDominators) {
    const assetPath = cleanPath(asset.filePath);
    contents.push(`"${assetPath}";`);
  }

  contents.push('');

  for (let asset of iterableDominators) {
    const assetPath = cleanPath(asset.filePath);
    const dominatorSetIds = dominators.getNodeIdsConnectedFrom(
      dominators.getNodeIdByContentKey(asset.id),
    );
    const dominatedAssets = [];
    dominatorSetIds.forEach((id) => {
      const node = dominators.getNode(id);
      if (node && node !== 'root') {
        dominatedAssets.push(node);
      }
    });

    const iterableDominatorSet = dominatedAssets.sort((a, b) =>
      cleanPath(a.filePath).localeCompare(cleanPath(b.filePath)),
    );

    for (let dominated of iterableDominatorSet) {
      if (dominated === asset) {
        continue;
      }

      const dominatedPath = cleanPath(dominated.filePath);
      contents.push(`"${assetPath}" -> "${dominatedPath}";`);
    }
  }

  return `
digraph dominators {
  labelloc="t";
  label="Dominators";

${contents.map((l) => (l.length > 0 ? `  ${l}` : '')).join('\n')}
}`.trim();
}

export function mergedDominatorsToDot(
  entryDir: string,
  dominators: PackagedDominatorGraph,
  label?: string = 'Merged',
): string {
  const contents = [];
  const cleanPath = (p) => {
    if (p.includes('esmodule-helpers.js')) {
      return 'esmodule_helpers.js';
    }
    return path.relative(entryDir, p);
  };

  const getIdentifier = (nodeId) => {
    const node = nullthrows(dominators.getNode(nodeId));
    if (node === 'root') {
      return '"root"';
    } else if (node.type === 'package') {
      return `"package_${node.id}"`;
    } else {
      return `"${cleanPath(node.filePath)}"`;
    }
  };

  dominators.traverse((nodeId) => {
    contents.push(`${getIdentifier(nodeId)};`);
  });

  contents.push('');

  dominators.traverse((nodeId) => {
    dominators.getNodeIdsConnectedFrom(nodeId).forEach((connectedNodeId) => {
      contents.push(
        `${getIdentifier(nodeId)} -> ${getIdentifier(connectedNodeId)};`,
      );
    });
  });

  return `
digraph merged {
  labelloc="t";
  label="${label}";
  layout="dot";

${contents.map((l) => (l.length > 0 ? `  ${l}` : '')).join('\n')}
}`.trim();
}

export function runDotForTest(
  __dirname: string,
  __filename: string,
  name: string,
  label: string,
  dot: string,
) {
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

export interface BundlerTestSetup {
  assetGraph: AssetGraph;
  mutableBundleGraph: MutableBundleGraph;
  bundleGraph: BundleGraph;
  entries: {|
    entryAsset: Asset,
    entryDependency: Dependency,
  |}[];
}

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
