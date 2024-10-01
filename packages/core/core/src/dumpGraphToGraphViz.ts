import type {Asset, BundleBehavior} from '@atlaspack/types';
import type {Graph} from '@atlaspack/graph';
import type {AssetGraphNode, BundleGraphNode, Environment} from './types';
import {bundleGraphEdgeTypes} from './BundleGraph';
import {requestGraphEdgeTypes} from './RequestTracker';

import path from 'path';
import {fromNodeId} from '@atlaspack/graph';
import {fromProjectPathRelative} from './projectPath';
import {SpecifierType, Priority} from './types';

const COLORS = {
  root: 'gray',
  asset: 'green',
  dependency: 'orange',
  transformer_request: 'cyan',
  file: 'gray',
  default: 'white',
} as const;

const TYPE_COLORS = {
  // bundle graph
  bundle: 'blue',
  contains: 'grey',
  internal_async: 'orange',
  references: 'red',
  sibling: 'green',
  // asset graph
  // request graph
  invalidated_by_create: 'green',
  invalidated_by_create_above: 'orange',
  invalidate_by_update: 'cyan',
  invalidated_by_delete: 'red',
} as const;

export default async function dumpGraphToGraphViz(
  graph:
    | Graph<AssetGraphNode>
    | Graph<{
        assets: Set<Asset>;
        sourceBundles: Set<number>;
        bundleBehavior?: BundleBehavior | null | undefined;
      }>
    | Graph<BundleGraphNode>,
  name: string,
  edgeTypes?: typeof bundleGraphEdgeTypes | typeof requestGraphEdgeTypes,
): Promise<void> {
  if (
    process.env.ATLASPACK_BUILD_ENV === 'production' &&
    !process.env.ATLASPACK_BUILD_REPL
  ) {
    return;
  }

  let mode: string | null | undefined = process.env.ATLASPACK_BUILD_REPL
    ? // $FlowFixMe
      // @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
      globalThis.ATLASPACK_DUMP_GRAPHVIZ?.mode
    : process.env.ATLASPACK_DUMP_GRAPHVIZ;

  // @ts-expect-error - TS2367 - This condition will always return 'false' since the types 'string' and 'boolean' have no overlap.
  if (mode == null || mode == false) {
    return;
  }

  let detailedSymbols = mode === 'symbols';

  let GraphVizGraph = require('graphviz/lib/deps/graph').Graph;
  let g = new GraphVizGraph(null, 'G');
  g.type = 'digraph';
  for (let [id, node] of graph.nodes.entries()) {
    if (node == null) continue;
    let n = g.addNode(nodeId(id));
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type '{ readonly root: "gray"; readonly asset: "green"; readonly dependency: "orange"; readonly transformer_request: "cyan"; readonly file: "gray"; readonly default: "white"; }'. | TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
    n.set('color', COLORS[node.type || 'default']);
    n.set('shape', 'box');
    n.set('style', 'filled');
    let label;
    if (typeof node === 'string') {
      label = node;
      // @ts-expect-error - TS2339 - Property 'assets' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
    } else if (node.assets) {
      // @ts-expect-error - TS2339 - Property 'assets' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      label = `(${nodeId(id)}), (assetIds: ${[...node.assets]
        .map((a) => {
          let arr = a.filePath.split('/');
          return arr[arr.length - 1];
        })
        // @ts-expect-error - TS2339 - Property 'sourceBundles' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        .join(', ')}) (sourceBundles: ${[...node.sourceBundles].join(
        ', ',
        // @ts-expect-error - TS2339 - Property 'bundleBehavior' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      )}) (bb ${node.bundleBehavior ?? 'none'})`;
      // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
    } else if (node.type) {
      // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'. | TS2339 - Property 'id' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      label = `[${fromNodeId(id)}] ${node.type || 'No Type'}: [${node.id}]: `;
      // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      if (node.type === 'dependency') {
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        label += node.value.specifier;
        let parts: Array<undefined | string> = [];
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.priority !== Priority.sync) {
          parts.push(
            Object.entries(Priority).find(
              // @ts-expect-error - TS2531 - Object is possibly 'null'. | TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
              ([, v]: [any, any]) => v === node.value.priority,
            )?.[0],
          );
        }
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.isOptional) parts.push('optional');
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.specifierType === SpecifierType.url) parts.push('url');
        // @ts-expect-error - TS2339 - Property 'hasDeferred' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.hasDeferred) parts.push('deferred');
        // @ts-expect-error - TS2339 - Property 'deferred' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.deferred) parts.push('deferred');
        // @ts-expect-error - TS2339 - Property 'excluded' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.excluded) parts.push('excluded');
        if (parts.length) label += ' (' + parts.join(', ') + ')';
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'. | TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.env) label += ` (${getEnvDescription(node.value.env)})`;
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        let depSymbols = node.value.symbols;
        if (detailedSymbols) {
          if (depSymbols) {
            if (depSymbols.size) {
              label +=
                '\\nsymbols: ' +
                [...depSymbols]
                  .map(([e, {local}]: [any, any]) => [e, local])
                  .join(';');
            }
            let weakSymbols = [...depSymbols]
              .filter(([, {isWeak}]: [any, any]) => isWeak)
              .map(([s]: [any]) => s);
            if (weakSymbols.length) {
              label += '\\nweakSymbols: ' + weakSymbols.join(',');
            }
            // @ts-expect-error - TS2339 - Property 'usedSymbolsUp' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
            if (node.usedSymbolsUp.size > 0) {
              label +=
                '\\nusedSymbolsUp: ' +
                // @ts-expect-error - TS2339 - Property 'usedSymbolsUp' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
                [...node.usedSymbolsUp]
                  .map(([s, sAsset]: [any, any]) =>
                    sAsset
                      ? `${s}(${sAsset.asset}.${sAsset.symbol ?? ''})`
                      : sAsset === null
                      ? `${s}(external)`
                      : `${s}(ambiguous)`,
                  )
                  .join(',');
            }
            // @ts-expect-error - TS2339 - Property 'usedSymbolsDown' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
            if (node.usedSymbolsDown.size > 0) {
              label +=
                // @ts-expect-error - TS2339 - Property 'usedSymbolsDown' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
                '\\nusedSymbolsDown: ' + [...node.usedSymbolsDown].join(',');
            }
            // if (node.usedSymbolsDownDirty) label += '\\nusedSymbolsDownDirty';
            // if (node.usedSymbolsUpDirtyDown)
            //   label += '\\nusedSymbolsUpDirtyDown';
            // if (node.usedSymbolsUpDirtyUp) label += '\\nusedSymbolsUpDirtyUp';
          } else {
            label += '\\nsymbols: cleared';
          }
        }
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      } else if (node.type === 'asset') {
        label +=
          // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          path.basename(fromProjectPathRelative(node.value.filePath)) +
          '#' +
          // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          node.value.type;
        if (detailedSymbols) {
          // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          if (!node.value.symbols) {
            label += '\\nsymbols: cleared';
            // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          } else if (node.value.symbols.size) {
            label +=
              '\\nsymbols: ' +
              // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
              [...node.value.symbols]
                .map(([e, {local}]: [any, any]) => [e, local])
                .join(';');
          }
          // @ts-expect-error - TS2339 - Property 'usedSymbols' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          if (node.usedSymbols.size) {
            // @ts-expect-error - TS2339 - Property 'usedSymbols' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
            label += '\\nusedSymbols: ' + [...node.usedSymbols].join(',');
          }
          // if (node.usedSymbolsDownDirty) label += '\\nusedSymbolsDownDirty';
          // if (node.usedSymbolsUpDirty) label += '\\nusedSymbolsUpDirty';
        } else {
          label += '\\nsymbols: cleared';
        }
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      } else if (node.type === 'asset_group') {
        // @ts-expect-error - TS2339 - Property 'deferred' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.deferred) label += '(deferred)';
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      } else if (node.type === 'file') {
        // @ts-expect-error - TS2339 - Property 'id' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        label += path.basename(node.id);
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      } else if (node.type === 'transformer_request') {
        label +=
          // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          path.basename(node.value.filePath) +
          // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
          ` (${getEnvDescription(node.value.env)})`;
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      } else if (node.type === 'bundle') {
        let parts: Array<string> = [];
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.needsStableName) parts.push('stable name');
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        parts.push(node.value.name);
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        parts.push('bb:' + (node.value.bundleBehavior ?? 'null'));
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.isPlaceholder) parts.push('placeholder');
        if (parts.length) label += ' (' + parts.join(', ') + ')';
        // @ts-expect-error - TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'. | TS2339 - Property 'value' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        if (node.value.env) label += ` (${getEnvDescription(node.value.env)})`;
        // @ts-expect-error - TS2339 - Property 'type' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
      } else if (node.type === 'request') {
        // @ts-expect-error - TS2339 - Property 'requestType' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'. | TS2339 - Property 'id' does not exist on type 'AssetNode | DependencyNode | RootNode | AssetGroupNode | EntrySpecifierNode | EntryFileNode | BundleGroupNode | BundleNode | { ...; }'.
        label = node.requestType + ':' + node.id;
      }
    }
    n.set('label', label);
  }

  let edgeNames;
  if (edgeTypes) {
    edgeNames = Object.fromEntries(
      Object.entries(edgeTypes).map(([k, v]: [any, any]) => [v, k]),
    );
  }

  // @ts-expect-error - TS2488 - Type 'Iterator<Edge<1>, any, undefined>' must have a '[Symbol.iterator]()' method that returns an iterator.
  for (let edge of graph.getAllEdges()) {
    let gEdge = g.addEdge(nodeId(edge.from), nodeId(edge.to));
    let color = null;
    if (edge.type != 1 && edgeNames) {
      // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type '{ readonly bundle: "blue"; readonly contains: "grey"; readonly internal_async: "orange"; readonly references: "red"; readonly sibling: "green"; readonly invalidated_by_create: "green"; readonly invalidated_by_create_above: "orange"; readonly invalidate_by_update: "cyan"; readonly invalidated_by_delete: "red"; }'.
      color = TYPE_COLORS[edgeNames[edge.type]];
    }
    if (color != null) {
      gEdge.set('color', color);
    }
  }

  if (process.env.ATLASPACK_BUILD_REPL) {
    // @ts-expect-error - TS7017 - Element implicitly has an 'any' type because type 'typeof globalThis' has no index signature.
    globalThis.ATLASPACK_DUMP_GRAPHVIZ?.(name, g.to_dot());
  } else {
    const tempy = require('tempy');
    let tmp = tempy.file({name: `parcel-${name}.png`});
    await g.output('png', tmp);
    // eslint-disable-next-line no-console
    console.log('Dumped', tmp);
  }
}

function nodeId(id: NodeId | number) {
  return `node${id}`;
}

function getEnvDescription(env: Environment) {
  let description;
  if (typeof env.engines.browsers === 'string') {
    description = `${env.context}: ${env.engines.browsers}`;
  } else if (Array.isArray(env.engines.browsers)) {
    description = `${env.context}: ${env.engines.browsers.join(', ')}`;
  } else if (env.engines.node) {
    description = `node: ${env.engines.node}`;
  } else if (env.engines.electron) {
    description = `electron: ${env.engines.electron}`;
  }

  return description ?? '';
}
