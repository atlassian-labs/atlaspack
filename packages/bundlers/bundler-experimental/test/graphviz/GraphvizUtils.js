// @flow strict-local

import childProcess from 'child_process';
import fs from 'fs';
import path from 'path';
import {ContentGraph} from '@atlaspack/graph';
import type {PackagedDominatorGraph} from '../../src/DominatorBundler';
import nullthrows from 'nullthrows';
import type {Asset, BundleGraph, Bundle} from '@atlaspack/types';

/**
 * Write a dot string to a file and generate a PNG using the `dot` CLI command.
 */
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
  fs.mkdirSync(slugTestName, {recursive: true});
  fs.mkdirSync(path.join(slugTestName, 'dot'), {recursive: true});
  fs.mkdirSync(path.join(slugTestName, 'png'), {recursive: true});
  const filePath = `${label}.dot`;
  fs.writeFileSync(path.join(slugTestName, 'dot', filePath), dot);
  childProcess.execSync(
    `dot -Tpng -o "${path.join(
      slugTestName,
      'png',
      filePath,
    )}.png" "${path.join(slugTestName, 'dot', filePath)}"`,
  );
}

/**
 * Create a dot string from a bundle graph.
 */
export function dotFromBundleGraph<B: Bundle>(
  entryDir: string,
  bundleGraph: BundleGraph<B>,
): string {
  const clean = (p) => cleanPath(entryDir, p);
  const contents = [];

  const bundles = bundleGraph.getBundles();

  for (let bundle of bundles) {
    const bundleId = bundle.id;
    contents.push(`subgraph cluster_${bundleId} {`);
    contents.push(`  label = "Bundle ${bundleId}";`);

    bundle.traverseAssets((asset) => {
      contents.push(`  "${clean(asset.filePath)}";`);
    });

    contents.push('}');
  }

  return `
digraph bundle_graph {
  labelloc="t";
  label="Bundle graph";

${contents.map((line) => (line.length > 0 ? `  ${line}` : '')).join('\n')}
}
  `.trim();
}

export function cleanPath(entryDir: string, p: string): string {
  if (p.includes('esmodule-helpers.js')) {
    return 'esmodule_helpers.js';
  }
  return path.relative(entryDir, p);
}

/**
 * Render a rooted graph to graphviz dot. The output is always on the same
 * order.
 */
export function rootedGraphToDot(
  entryDir: string,
  dominators: ContentGraph<Asset | 'root'>,
  label?: string = 'Dominators',
  name?: string = 'dominators',
): string {
  const contents = [];
  const clean = (p) => cleanPath(entryDir, p);

  contents.push('"root";');
  const rootNodeId = dominators.getNodeIdByContentKey('root');
  const rootNodes = dominators
    .getNodeIdsConnectedFrom(rootNodeId)
    .map((id) => {
      const node = dominators.getNode(id);
      if (node && node !== 'root') {
        return clean(node.filePath);
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
    clean(a.filePath).localeCompare(clean(b.filePath)),
  );

  for (let asset of iterableDominators) {
    const assetPath = clean(asset.filePath);
    contents.push(`"${assetPath}";`);
  }

  contents.push('');

  for (let asset of iterableDominators) {
    const assetPath = clean(asset.filePath);
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
      clean(a.filePath).localeCompare(clean(b.filePath)),
    );

    for (let dominated of iterableDominatorSet) {
      if (dominated === asset) {
        continue;
      }

      const dominatedPath = clean(dominated.filePath);
      contents.push(`"${assetPath}" -> "${dominatedPath}";`);
    }
  }

  return `
digraph ${name} {
  labelloc="t";
  label="${label}";

${contents.map((l) => (l.length > 0 ? `  ${l}` : '')).join('\n')}
}`.trim();
}

/**
 * Render the packaged dominator tree to graphviz dot
 */
export function mergedDominatorsToDot(
  entryDir: string,
  dominators: PackagedDominatorGraph,
  label?: string = 'Merged',
): string {
  const contents = [];
  const getIdentifier = (nodeId) => {
    const node = nullthrows(dominators.getNode(nodeId));
    if (node === 'root') {
      return '"root"';
    } else if (node.type === 'package') {
      return `"package_${node.id}"`;
    } else {
      return `"${cleanPath(entryDir, node.filePath)}"`;
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
