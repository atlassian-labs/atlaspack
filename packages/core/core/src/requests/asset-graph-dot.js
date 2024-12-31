// @flow
import path from 'path';

import type {AssetGraphNode} from '../types';
import AssetGraph from '../AssetGraph';

export type AssetGraphToDotOptions = {
  style?: boolean,
  sort?: boolean,
  ...
};

/** @description Renders AssetGraph into GraphViz Dot format */
export function assetGraphToDot(
  assetGraph: AssetGraph,
  {sort = false, style = true}: AssetGraphToDotOptions = {},
): string {
  const edges = [];
  const nodeStyles: {[key: string]: string} = {};

  assetGraph.traverse((nodeId) => {
    let node: AssetGraphNode | null = assetGraph.getNode(nodeId) ?? null;
    if (!node) return;

    const fromIds = assetGraph.getNodeIdsConnectedTo(nodeId);

    for (const fromId of fromIds) {
      let fromNode: AssetGraphNode | null = assetGraph.getNode(fromId) ?? null;
      if (!fromNode) throw new Error('No Node');
      const edgeStyle = getEdgeStyle(node);
      const nodeStyle = getNodeStyle(node);

      let entry = `"${getNodeName(fromNode)}" -> "${getNodeName(node)}"`;
      if (edgeStyle) {
        entry += ` [${edgeStyle}]`;
      }

      if (nodeStyle) {
        nodeStyles[getNodeName(node)] = nodeStyle;
      }

      edges.push(entry);
    }
  });

  // $FlowFixMe
  const nodeStylesList: Array<[string, string]> = Object.entries(nodeStyles);
  if (sort) {
    edges.sort();
    nodeStylesList.sort();
  }

  let digraph = `digraph {\n\tnode [shape=rectangle]\n`;
  if (style) {
    digraph += nodeStylesList
      .map(([node, style]) => `\t"${node}" [${style}]\n`)
      .join('');
  }
  digraph += edges.map((v) => `\t${v}\n`).join('');
  digraph += `}\n`;
  return digraph;
}

export function getDebugAssetGraphDotPath(): string | null {
  let debugAssetGraphDot = process.env.DEBUG_ASSET_GRAPH_DOT;
  if (debugAssetGraphDot === undefined || debugAssetGraphDot === '') {
    return null;
  }
  if (!path.isAbsolute(debugAssetGraphDot)) {
    debugAssetGraphDot = path.join(process.cwd(), debugAssetGraphDot);
  }
  return debugAssetGraphDot;
}

export function getDebugAssetGraphDotOptions(): AssetGraphToDotOptions {
  const options: AssetGraphToDotOptions = {};

  let style = process.env.DEBUG_ASSET_GRAPH_DOT_STYLE;
  if (style !== undefined) {
    options.style = style === 'true';
  }

  let sort = process.env.DEBUG_ASSET_GRAPH_DOT_SORT;
  if (sort !== undefined) {
    options.sort = sort === 'true';
  }

  return options;
}

function fromCwd(input: string): string {
  return path.relative(process.cwd(), input);
}

function getNodeName(node: AssetGraphNode): string {
  if (node.type === 'asset_group') {
    // $FlowFixMe
    return [`asset_group`, node.id, fromCwd(node.value.filePath)].join('\\n');
  } else if (node.type === 'asset') {
    // $FlowFixMe
    return [`asset`, node.id, fromCwd(node.value.filePath)].join('\\n');
  } else if (node.type === 'dependency') {
    return [`dependency`, node.id, node.value.specifier].join('\\n');
  } else if (node.type === 'entry_specifier') {
    // $FlowFixMe
    return [`entry_specifier`, node.value].join('\\n');
  } else if (node.type === 'entry_file') {
    // $FlowFixMe
    return [`entry_file`, fromCwd(node.value.filePath)].join('\\n');
  }
  return 'ROOT';
}

function getEdgeStyle(node: AssetGraphNode): string {
  if (node.type === 'asset_group') {
    return ``;
  } else if (node.type === 'asset') {
    return ``;
  } else if (node.type === 'dependency') {
    if (node.value.priority === 2) {
      return `style="dashed"`;
    }
  } else if (node.type === 'entry_specifier') {
    return ``;
  } else if (node.type === 'entry_file') {
    return ``;
  }
  return '';
}

function getNodeStyle(node: AssetGraphNode): string {
  if (node.type === 'asset_group') {
    return `fillcolor="#E8F5E9", style="filled"`;
  } else if (node.type === 'asset') {
    return `fillcolor="#DCEDC8", style="filled"`;
  } else if (node.type === 'dependency') {
    return `fillcolor="#BBDEFB", style="filled"`;
  } else if (node.type === 'entry_specifier') {
    return `fillcolor="#FFF9C4", style="filled"`;
  } else if (node.type === 'entry_file') {
    return `fillcolor="#FFE0B2", style="filled"`;
  }
  return '';
}
