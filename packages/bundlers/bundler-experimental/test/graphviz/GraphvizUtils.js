// @flow strict-local

import childProcess from 'child_process';
import fs from 'fs';
import path from 'path';
import type {PackagedDominatorGraph} from '../../src/DominatorBundler/createPackages';
import nullthrows from 'nullthrows';
import type {BundleGraph, Bundle} from '@atlaspack/types';
import type {
  AcyclicGraph,
  StronglyConnectedComponentNode,
} from '../../src/DominatorBundler/cycleBreaker';
import type {
  AssetNode,
  SimpleAssetGraph,
  SimpleAssetGraphEdgeWeight,
  SimpleAssetGraphNode,
} from '../../src/DominatorBundler/bundleGraphToRootedGraph';
import invariant from 'assert';

/**
 * Write a dot string to a file and generate a SVG using the `dot` CLI command.
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
  fs.mkdirSync(path.join(slugTestName, 'svg'), {recursive: true});
  const filePath = `${label}.dot`;
  fs.writeFileSync(path.join(slugTestName, 'dot', filePath), dot);
  childProcess.execSync(
    `dot -Tsvg -o "${path.join(
      slugTestName,
      'svg',
      filePath,
    )}.svg" "${path.join(slugTestName, 'dot', filePath)}"`,
  );
}

export function dotFromBundleGroupsInGraph<B: Bundle>(
  entryDir: string,
  bundleGraph: BundleGraph<B>,
): string {
  const contents = [];

  const bundleGroups = new Set(
    bundleGraph.getBundles({includeInline: true}).flatMap((bundle) => {
      return bundleGraph.getBundleGroupsContainingBundle(bundle);
    }),
  );

  for (let bundleGroup of bundleGroups) {
    const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup, {
      includeInline: true,
    });
    const id = bundleGroup.entryAssetId;
    contents.push(`subgraph cluster_${id} {`);
    contents.push(`  label = "Bundle Group ${id}";`);

    for (let bundle of bundles) {
      contents.push(`  "${bundle.id}";`);
    }

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
      if (asset.filePath.includes('esmodule-helpers.js')) {
        return;
      }
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
    return 'esmodule-helpers.js';
  }
  return path.relative(entryDir, p);
}

/**
 * Render a rooted graph to graphviz dot. The output is always on the same
 * order.
 */
export function rootedGraphToDot(
  entryDir: string,
  dominators:
    | AcyclicGraph<SimpleAssetGraphNode, SimpleAssetGraphEdgeWeight>
    | SimpleAssetGraph,
  label: string = 'Dominators',
  name: string = 'dominators',
): string {
  const contents = [];
  const clean = (p: string) => cleanPath(entryDir, p);
  const getLabel = (
    node:
      | SimpleAssetGraphNode
      | StronglyConnectedComponentNode<SimpleAssetGraphNode>,
  ) => {
    if (node == null || node === 'root') {
      return 'root';
    }

    if (node.type === 'StronglyConnectedComponent') {
      return 'SCC';
    }

    return clean(node.asset.filePath);
  };

  contents.push('"root";');
  const rootNodeId = dominators.getNodeIdByContentKey('root');
  const rootNodes = dominators
    .getNodeIdsConnectedFrom(rootNodeId)
    .map((id) => {
      const node = dominators.getNode(id);
      invariant(node != null && node !== 'root');
      if (
        node.type === 'asset' &&
        node.asset.filePath.includes('esmodule-helpers.js')
      ) {
        return;
      }
      return getLabel(node);
    })
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b));

  rootNodes.forEach((node) => {
    contents.push(`"root" -> "${node}";`);
  });

  const iterableDominators: (
    | AssetNode
    | StronglyConnectedComponentNode<AssetNode | 'root'>
  )[] = [];

  // $FlowFixMe
  dominators.nodes.forEach((node) => {
    if (node != null && node !== 'root') {
      iterableDominators.push(node);
    }
  });

  iterableDominators.sort((a, b) => getLabel(a).localeCompare(getLabel(b)));

  for (let asset of iterableDominators) {
    const assetPath = getLabel(asset);
    if (assetPath.includes('esmodule-helpers.js')) {
      continue;
    }
    contents.push(`"${assetPath}";`);
  }

  contents.push('');

  for (let asset of iterableDominators) {
    const assetPath = getLabel(asset);
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
      getLabel(a).localeCompare(getLabel(b)),
    );

    for (let dominated of iterableDominatorSet) {
      if (dominated === asset) {
        continue;
      }

      const dominatedPath = getLabel(dominated);
      if (dominatedPath.includes('esmodule-helpers.js')) {
        continue;
      }
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
  label: string = 'Merged',
): string {
  const contents = [];
  const getIdentifier = (nodeId) => {
    const node = nullthrows(dominators.getNode(nodeId));
    if (node === 'root') {
      return '"root"';
    } else if (node.type === 'package') {
      return `"${node.id}"`;
    } else if (node.type === 'StronglyConnectedComponent') {
      return `"scc_${node.id}"`;
    } else {
      return `"${cleanPath(entryDir, node.asset.filePath)}"`;
    }
  };

  dominators.traverse((nodeId) => {
    const identifier = getIdentifier(nodeId);
    if (identifier.includes('esmodule-helpers.js')) {
      return;
    }
    contents.push(`${identifier};`);
  });
  contents.sort((a, b) => a.localeCompare(b));

  contents.push('');

  const connections = [];
  dominators.traverse((nodeId) => {
    dominators.getNodeIdsConnectedFrom(nodeId).forEach((connectedNodeId) => {
      const source = getIdentifier(nodeId);
      const target = getIdentifier(connectedNodeId);
      if (
        source.includes('esmodule-helpers.js') ||
        target.includes('esmodule-helpers.js')
      ) {
        return;
      }
      connections.push(`${source} -> ${target};`);
    });
  });
  connections.sort((a, b) => a.localeCompare(b));

  contents.push(...connections);

  return `
digraph merged {
  labelloc="t";
  label="${label}";
  layout="dot";

${contents.map((l) => (l.length > 0 ? `  ${l}` : '')).join('\n')}
}`.trim();
}
