// @flow

/**
 * In the functions, the direction is set from the root, down refers to
 *
 * */

import type {ContentKey, NodeId} from '@atlaspack/graph';
import type {Meta, Symbol} from '@atlaspack/types';
import type {Diagnostic} from '@atlaspack/diagnostic';
import type {
  AssetNode,
  DependencyNode,
  InternalSourceLocation,
  AtlaspackOptions,
} from './types';
import {type default as AssetGraph} from './AssetGraph';

import invariant from 'assert';
import nullthrows from 'nullthrows';
import {setEqual} from '@atlaspack/utils';
import logger from '@atlaspack/logger';
import {md, convertSourceLocationToHighlight} from '@atlaspack/diagnostic';
import {BundleBehavior} from './types';
import {fromProjectPathRelative, fromProjectPath} from './projectPath';

function logFallbackNamespaceInsertion(
  assetNode,
  symbol: Symbol,
  depNode1,
  depNode2,
  options: AtlaspackOptions,
) {
  if (options.logLevel === 'verbose') {
    logger.warn({
      message: `${fromProjectPathRelative(
        assetNode.value.filePath,
      )} reexports "${symbol}", which could be resolved either to the dependency "${
        depNode1.value.specifier
      }" or "${
        depNode2.value.specifier
      }" at runtime. Adding a namespace object to fall back on.`,
      origin: '@atlaspack/core',
    });
  }
}

/*
 * Incoming dependencies of an asset are the assets imports, so what the asset needs
 * Outgoing dependencies of an asset are the assets exports, so what the asset provides
 * The direction of traversal in the code is relative to the root
 * Going down means going from an asset to it's dependencies, there is no down from the leaf nodes of the asset graph
 * Going up means going towards assets that depend on the current asset, there is no up from the root
 */
export function propagateSymbols({
  options,
  assetGraph,
  changedAssetsPropagation,
  assetGroupsWithRemovedParents,
  previousErrors,
}: {|
  options: AtlaspackOptions,
  assetGraph: AssetGraph,
  changedAssetsPropagation: Set<string>,
  assetGroupsWithRemovedParents: Set<NodeId>,
  previousErrors?: ?Map<NodeId, Array<Diagnostic>>,
|}): Map<NodeId, Array<Diagnostic>> {
  const changedAssets = getChangedAssets(assetGraph, changedAssetsPropagation);

  const changedDeps = new Set<DependencyNode>();

  const changedDepsUsedSymbolsUpDirtyDown = new Set<ContentKey>();

  /*
   * To propagate symbols from an asset to the incoming dependencies we
   * - get the export symbol identifier map
   * - we invert it as it is more convenient to have a map of identifiers leading to the corresponding export symbols
   * - we find dependency requests corresponding to imports made by the asset
   * - we distribute the exported symbols to assets importing the current asset
   */
  propagateSymbolsDown(
    assetGraph,
    changedAssets,
    assetGroupsWithRemovedParents,
    (assetNode, incomingDeps, outgoingDeps) => {
      const exportSymbolToIdentifierMap: ?$ReadOnlyMap<
        Symbol,
        {|local: Symbol, loc: ?InternalSourceLocation, meta?: ?Meta|},
      > = assetNode.value.symbols;

      const identifierToExportSymbolMap = exportSymbolToIdentifierMap
        ? invertAssetSymbolsMapping(exportSymbolToIdentifierMap)
        : null;

      const {isEntry, isEverySymbolCleared, namespaceReexportedSymbols} =
        getIncomingDependencyRequestsFromAsset(
          assetNode,
          incomingDeps,
          exportSymbolToIdentifierMap,
          outgoingDeps,
        );

      distributeSymbolsToOutgoingDependencies(
        outgoingDeps,
        assetNode,
        isEverySymbolCleared,
        isEntry,
        namespaceReexportedSymbols,
        identifierToExportSymbolMap,
        changedDepsUsedSymbolsUpDirtyDown,
      );
    },
  );

  const errors = propagateSymbolsUp(
    assetGraph,
    changedAssets,
    changedDepsUsedSymbolsUpDirtyDown,
    previousErrors,
    (assetNode, incomingDeps, outgoingDeps) => {
      const exportSymbolToIdentifierMap: ?$ReadOnlyMap<
        Symbol,
        {|local: Symbol, loc: ?InternalSourceLocation, meta?: ?Meta|},
      > = assetNode.value.symbols;
      const identifierToExportSymbolMap = exportSymbolToIdentifierMap
        ? invertAssetSymbolsMapping(exportSymbolToIdentifierMap)
        : null;

      const reexportedSymbols = new Map<
        Symbol,
        ?{|asset: ContentKey, symbol: ?Symbol|},
      >();
      const reexportedSymbolsSource = new Map<Symbol, DependencyNode>();

      processOutgoingDependencies(
        assetGraph,
        outgoingDeps,
        identifierToExportSymbolMap,
        reexportedSymbols,
        reexportedSymbolsSource,
        options,
        assetNode,
      );
      return processIncomingDependencies(
        incomingDeps,
        assetNode,
        exportSymbolToIdentifierMap,
        reexportedSymbols,
        reexportedSymbolsSource,
        assetGraph,
        options,
        changedDeps,
      );
    },
  );

  sortUsedSymbolsUp(changedDeps);

  return errors;
}

function distributeSymbolsToOutgoingDependencies(
  outgoingDeps,
  assetNode,
  isEverySymbolCleared,
  isEntry,
  namespaceReexportedSymbols,
  identifierToExportSymbolMap,
  changedDepsUsedSymbolsUpDirtyDown,
) {
  for (let dep of outgoingDeps) {
    let depUsedSymbolsDownOld = dep.usedSymbolsDown;
    let depUsedSymbolsDown = new Set();
    dep.usedSymbolsDown = depUsedSymbolsDown;
    if (
      shouldPropagateSymbols(
        assetNode,
        isEverySymbolCleared,
        isEntry,
        namespaceReexportedSymbols,
      )
    ) {
      processOutgoingDependencySymbols(
        dep,
        depUsedSymbolsDown,
        namespaceReexportedSymbols,
        isEverySymbolCleared,
        assetNode,
        identifierToExportSymbolMap,
      );
    } else {
      depUsedSymbolsDown.clear();
    }
    updateDependencyState(
      dep,
      depUsedSymbolsDownOld,
      depUsedSymbolsDown,
      changedDepsUsedSymbolsUpDirtyDown,
    );
  }
}
function shouldPropagateSymbols(
  assetNode,
  isEverySymbolCleared,
  isEntry,
  namespaceReexportedSymbols,
) {
  return (
    assetNode.value.sideEffects ||
    isEverySymbolCleared ||
    isEntry ||
    assetNode.usedSymbols.size > 0 ||
    namespaceReexportedSymbols.size > 0
  );
}
function processOutgoingDependencySymbols(
  dep,
  depUsedSymbolsDown,
  namespaceReexportedSymbols,
  isEverySymbolCleared,
  assetNode,
  identifierToExportSymbolMap,
) {
  let depSymbols = dep.value.symbols;
  if (!depSymbols) return;
  handleNamespaceSymbols(
    depUsedSymbolsDown,
    depSymbols,
    namespaceReexportedSymbols,
    isEverySymbolCleared,
  );
  for (let [symbol, {local}] of depSymbols) {
    if (local === '*') continue;
    if (!identifierToExportSymbolMap || !depSymbols.get(symbol)?.isWeak) {
      depUsedSymbolsDown.add(symbol);
    } else {
      handleReexportedSymbols(
        depUsedSymbolsDown,
        symbol,
        local,
        assetNode,
        identifierToExportSymbolMap,
      );
    }
  }
}
function handleNamespaceSymbols(
  depUsedSymbolsDown,
  depSymbols,
  namespaceReexportedSymbols,
  isEverySymbolCleared,
) {
  if (depSymbols.get('*')?.local === '*') {
    if (isEverySymbolCleared) {
      depUsedSymbolsDown.add('*');
    } else {
      namespaceReexportedSymbols.forEach(s => depUsedSymbolsDown.add(s));
    }
  }
}
function handleReexportedSymbols(
  depUsedSymbolsDown,
  symbol,
  local,
  assetNode,
  identifierToExportSymbolMap,
) {
  let reexportedExportSymbols = identifierToExportSymbolMap.get(local);
  if (reexportedExportSymbols == null) {
    depUsedSymbolsDown.add(symbol);
  } else if (assetNode.usedSymbols.has('*')) {
    depUsedSymbolsDown.add(symbol);
    reexportedExportSymbols.forEach(s => assetNode.usedSymbols.delete(s));
  } else {
    let usedReexportedExportSymbols = [...reexportedExportSymbols].filter(s =>
      assetNode.usedSymbols.has(s),
    );
    if (usedReexportedExportSymbols.length > 0) {
      depUsedSymbolsDown.add(symbol);
      usedReexportedExportSymbols.forEach(s => assetNode.usedSymbols.delete(s));
    }
  }
}
function updateDependencyState(
  dep,
  depUsedSymbolsDownOld,
  depUsedSymbolsDown,
  changedDepsUsedSymbolsUpDirtyDown,
) {
  if (!setEqual(depUsedSymbolsDownOld, depUsedSymbolsDown)) {
    dep.usedSymbolsDownDirty = true;
    dep.usedSymbolsUpDirtyDown = true;
    changedDepsUsedSymbolsUpDirtyDown.add(dep.id);
  }
  if (dep.usedSymbolsUpDirtyDown) {
    changedDepsUsedSymbolsUpDirtyDown.add(dep.id);
  }
}

function getIncomingDependencyRequestsFromAsset(
  assetNode,
  incomingDeps,
  exportSymbolToIdentifierMap,
  outgoingDeps,
) {
  const ROOT_SYMBOL = '*';
  let hasNamespaceOutgoingDeps = checkForNamespaceOutgoingDeps(
    outgoingDeps,
    ROOT_SYMBOL,
  );
  let isEntryAsset = false;
  let shouldAddAllSymbols = false;
  // Initialize asset's used symbols
  assetNode.usedSymbols = new Set();
  let namespaceReexportedSymbols = new Set<Symbol>();
  if (incomingDeps.length === 0) {
    handleRootAsset(assetNode, namespaceReexportedSymbols, ROOT_SYMBOL);
  } else {
    for (let incomingDep of incomingDeps) {
      if (incomingDep.value.symbols == null) {
        ({isEntryAsset, shouldAddAllSymbols} = handleClearedSymbolsDependency(
          incomingDep,
          isEntryAsset,
          shouldAddAllSymbols,
        ));
        continue;
      }
      processIncomingDependencySymbols(
        incomingDep,
        assetNode,
        exportSymbolToIdentifierMap,
        hasNamespaceOutgoingDeps,
        namespaceReexportedSymbols,
        ROOT_SYMBOL,
      );
    }
  }
  if (shouldAddAllSymbols) {
    addAllSymbols(exportSymbolToIdentifierMap, assetNode);
  }
  return {
    isEntry: isEntryAsset,
    isEverySymbolCleared: shouldAddAllSymbols,
    namespaceReexportedSymbols,
  };
}
function checkForNamespaceOutgoingDeps(outgoingDeps, rootSymbol) {
  return outgoingDeps.some(
    dep => dep.value.symbols?.get(rootSymbol)?.local === rootSymbol,
  );
}
function handleRootAsset(assetNode, namespaceReexportedSymbols, rootSymbol) {
  assetNode.usedSymbols.add(rootSymbol);
  namespaceReexportedSymbols.add(rootSymbol);
}
function handleClearedSymbolsDependency(
  incomingDep,
  isEntryAsset,
  shouldAddAllSymbols,
) {
  if (incomingDep.value.sourceAssetId == null) {
    isEntryAsset = true;
  } else {
    shouldAddAllSymbols = true;
  }
  return {isEntryAsset, shouldAddAllSymbols};
}
function processIncomingDependencySymbols(
  incomingDep,
  assetNode,
  exportSymbolToIdentifierMap,
  hasNamespaceOutgoingDeps,
  namespaceReexportedSymbols,
  rootSymbol,
) {
  for (let exportSymbol of incomingDep.usedSymbolsDown) {
    if (exportSymbol === rootSymbol) {
      markAsRootAsset(assetNode, rootSymbol);
      continue;
    }
    const isOwnSymbolOrNonNamespaceReexport =
      !exportSymbolToIdentifierMap ||
      exportSymbolToIdentifierMap.has(exportSymbol) ||
      exportSymbolToIdentifierMap.has(rootSymbol);
    if (isOwnSymbolOrNonNamespaceReexport) {
      assetNode.usedSymbols.add(exportSymbol);
    } else if (hasNamespaceOutgoingDeps && exportSymbol !== 'default') {
      namespaceReexportedSymbols.add(exportSymbol);
    }
  }
}
function addAllSymbols(exportSymbolToIdentifierMap, assetNode) {
  exportSymbolToIdentifierMap?.forEach((_, exportSymbol) =>
    assetNode.usedSymbols.add(exportSymbol),
  );
}
function markAsRootAsset(asset, ROOT_SYMBOL) {
  asset.usedSymbols.add(ROOT_SYMBOL);
  asset.namespaceReexportedSymbols.add(ROOT_SYMBOL);
}

function propagateSymbolsDown(
  assetGraph: AssetGraph,
  changedAssets: Set<NodeId>,
  assetGroupsWithRemovedParents: Set<NodeId>,
  visit: (
    assetNode: AssetNode,
    incoming: $ReadOnlyArray<DependencyNode>,
    outgoing: $ReadOnlyArray<DependencyNode>,
  ) => void,
) {
  if (changedAssets.size === 0 && assetGroupsWithRemovedParents.size === 0) {
    return;
  }
  let unreachedNodes = initializeUnreachedNodes(
    changedAssets,
    assetGroupsWithRemovedParents,
  );
  let processingQueue = new Set([setPop(unreachedNodes)]);
  while (processingQueue.size > 0) {
    let currentNodeId = setPop(processingQueue);
    unreachedNodes.delete(currentNodeId);
    let outgoingDependencies =
      assetGraph.getNodeIdsConnectedFrom(currentNodeId);
    let currentNode = nullthrows(assetGraph.getNode(currentNodeId));
    let isCurrentNodeDirty = processCurrentNode(
      assetGraph,
      currentNode,
      outgoingDependencies,
      visit,
    );
    enqueueDirtyChildNodes(
      assetGraph,
      processingQueue,
      currentNode,
      isCurrentNodeDirty,
      outgoingDependencies,
    );
    if (processingQueue.size === 0 && unreachedNodes.size > 0) {
      processingQueue.add(setPop(unreachedNodes));
    }
  }
}

function propagateSymbolsUp(
  assetGraph: AssetGraph,
  changedAssets: Set<NodeId>,
  changedDepsUsedSymbolsUpDirtyDown: Set<ContentKey>,
  previousBuildErrors: ?Map<NodeId, Array<Diagnostic>>,
  visit: (
    assetNode: AssetNode,
    incoming: $ReadOnlyArray<DependencyNode>,
    outgoing: $ReadOnlyArray<DependencyNode>,
  ) => Array<Diagnostic>,
): Map<NodeId, Array<Diagnostic>> {
  let errors = getPreviousBuildErrorsStillPresent(
    assetGraph,
    previousBuildErrors,
  );
  let dirtyNodes = findDirtyNodes(
    assetGraph,
    changedAssets,
    changedDepsUsedSymbolsUpDirtyDown,
  );

  if (shouldRunFullPass(assetGraph, dirtyNodes)) {
    dirtyNodes = performFullTraversal(assetGraph, visit, errors);
  }

  processDirtyNodes(assetGraph, dirtyNodes, visit, errors);

  return errors;
}

function getPreviousBuildErrorsStillPresent(assetGraph, previousBuildErrors) {
  return previousBuildErrors
    ? new Map(
        [...previousBuildErrors].filter(([nodeId]) =>
          assetGraph.hasNode(nodeId),
        ),
      )
    : new Map();
}

function findDirtyNodes(
  assetGraph,
  changedAssets,
  changedDepsUsedSymbolsUpDirtyDown,
) {
  return new Set(
    [...changedDepsUsedSymbolsUpDirtyDown]
      .reverse()
      .flatMap(id => getDependencyResolution(assetGraph, id)),
    ...changedAssets,
  );
}

function getDependencyResolution(
  graph: AssetGraph,
  depId: ContentKey,
): Array<NodeId> {
  let depNodeId = graph.getNodeIdByContentKey(depId);
  let connected = graph.getNodeIdsConnectedFrom(depNodeId);
  invariant(connected.length <= 1);
  let child = connected[0];
  if (child) {
    let childNode = nullthrows(graph.getNode(child));
    if (childNode.type === 'asset_group') {
      return graph.getNodeIdsConnectedFrom(child);
    } else {
      return [child];
    }
  }
  return [];
}

const ACCEPTABLE_RATIO_OF_DIRTY_NODES_IN_ASSET_GRAPH = 0.5;

const EXPECTED_NODES_PER_ASSET_RATIO = 6;

function shouldRunFullPass(assetGraph, dirtyNodes) {
  const estimatedAssetCount =
    assetGraph.nodes.length / EXPECTED_NODES_PER_ASSET_RATIO;
  return (
    estimatedAssetCount * ACCEPTABLE_RATIO_OF_DIRTY_NODES_IN_ASSET_GRAPH <
    dirtyNodes.size
  );
}

function performFullTraversal(assetGraph, visit, errors) {
  let dirtyDeps = new Set();
  let rootNodeId = nullthrows(
    assetGraph.rootNodeId,
    'A root node is required to traverse',
  );

  function visitNode(nodeId) {
    let node = nullthrows(assetGraph.getNode(nodeId));
    let outgoingDeps = getOutgoingDependencies(assetGraph, nodeId);

    propagateUsedSymbols(node, outgoingDeps, 'up');
    if (node.type === 'asset') {
      let incomingDeps = getIncomingDependencies(assetGraph, node);
      propagateUsedSymbols(node, incomingDeps, 'down');
      handleNodeVisit(node, incomingDeps, outgoingDeps, visit, errors, nodeId);
    } else if (node.type === 'dependency') {
      updateDirtyDependencies(node, nodeId, dirtyDeps);
    }
  }

  assetGraph.postOrderDfsFast(visitNode, rootNodeId);
  return dirtyDeps;
}

function processDirtyNodes(assetGraph, queue, visit, errors) {
  while (queue.size > 0) {
    let queuedNodeId = setPop(queue);
    let node = nullthrows(assetGraph.getNode(queuedNodeId));

    if (node.type === 'asset') {
      let incomingDeps = getIncomingDependencies(assetGraph, node);
      propagateUsedSymbols(node, incomingDeps, 'down');
      let outgoingDeps = getOutgoingDependencies(assetGraph, queuedNodeId);
      propagateUsedSymbols(node, outgoingDeps, 'up');

      handleNodeVisit(
        node,
        incomingDeps,
        outgoingDeps,
        visit,
        errors,
        queuedNodeId,
      );
      enqueueDirtyDependencies(incomingDeps, queue, assetGraph);
    } else {
      let connectedNodes = assetGraph.getNodeIdsConnectedTo(queuedNodeId);
      if (connectedNodes.length > 0) {
        queue.add(...connectedNodes);
      }
    }
  }
}

function getOutgoingDependencies(assetGraph, nodeId) {
  return assetGraph.getNodeIdsConnectedFrom(nodeId).map(depNodeId => {
    let depNode = nullthrows(assetGraph.getNode(depNodeId));
    invariant(depNode.type === 'dependency');
    return depNode;
  });
}

function propagateUsedSymbols(node, dependencies, direction) {
  for (let dep of dependencies) {
    if (
      (direction === 'down' && dep.usedSymbolsUpDirtyDown) ||
      (direction === 'up' && dep.usedSymbolsUpDirtyUp)
    ) {
      node.usedSymbolsUpDirty = true;
      if (direction === 'down') {
        dep.usedSymbolsUpDirtyDown = false;
      } else {
        dep.usedSymbolsUpDirtyUp = false;
      }
    }
  }
}

function handleNodeVisit(
  node,
  incomingDeps,
  outgoingDeps,
  visit,
  errors,
  nodeId,
) {
  if (node.usedSymbolsUpDirty) {
    let diagnostics = visit(node, incomingDeps, outgoingDeps);
    updateErrorsAndState(node, diagnostics, nodeId, errors);
  }
}

function updateErrorsAndState(node, diagnostics, nodeId, errors) {
  if (diagnostics.length > 0) {
    node.usedSymbolsUpDirty = true;
    errors.set(nodeId, diagnostics);
  } else {
    node.usedSymbolsUpDirty = false;
    errors.delete(nodeId);
  }
}

function initializeUnreachedNodes(
  changedAssets,
  assetGroupsWithRemovedParents,
) {
  return new Set([...changedAssets, ...assetGroupsWithRemovedParents]);
}
function processCurrentNode(
  assetGraph,
  currentNode,
  outgoingDependencies,
  visit,
) {
  let wasNodeDirty = false;
  if (currentNode.type === 'dependency' || currentNode.type === 'asset_group') {
    wasNodeDirty = currentNode.usedSymbolsDownDirty;
    currentNode.usedSymbolsDownDirty = false;
  } else if (currentNode.type === 'asset' && currentNode.usedSymbolsDownDirty) {
    visit(
      currentNode,
      getIncomingDependencies(assetGraph, currentNode),
      getOutgoingDependencyNodes(assetGraph, outgoingDependencies),
    );
    currentNode.usedSymbolsDownDirty = false;
  }
  return wasNodeDirty;
}
function getIncomingDependencies(assetGraph, assetNode) {
  return assetGraph.getIncomingDependencies(assetNode.value).map(dependency => {
    let dependencyNode = assetGraph.getNodeByContentKey(dependency.id);
    invariant(dependencyNode && dependencyNode.type === 'dependency');
    return dependencyNode;
  });
}
function getOutgoingDependencyNodes(assetGraph, outgoingDependencies) {
  return outgoingDependencies.map(dependencyId => {
    let dependencyNode = nullthrows(assetGraph.getNode(dependencyId));
    invariant(dependencyNode.type === 'dependency');
    return dependencyNode;
  });
}

function updateDirtyDependencies(node, nodeId, dirtyDeps) {
  if (node.usedSymbolsUpDirtyUp) {
    dirtyDeps.add(nodeId);
  } else {
    dirtyDeps.delete(nodeId);
  }
}

function enqueueDirtyDependencies(incomingDeps, queue, assetGraph) {
  for (let dep of incomingDeps) {
    if (dep.usedSymbolsUpDirtyUp) {
      queue.add(assetGraph.getNodeIdByContentKey(dep.id));
    }
  }
}

function enqueueDirtyChildNodes(
  assetGraph,
  processingQueue,
  currentNode,
  wasNodeDirty,
  outgoingDependencies,
) {
  for (let childId of outgoingDependencies) {
    let childNode = nullthrows(assetGraph.getNode(childId));
    let isChildDirty = false;
    if (
      (childNode.type === 'asset' || childNode.type === 'asset_group') &&
      wasNodeDirty
    ) {
      childNode.usedSymbolsDownDirty = true;
      isChildDirty = true;
    } else if (childNode.type === 'dependency') {
      isChildDirty = childNode.usedSymbolsDownDirty;
    }
    if (isChildDirty) {
      processingQueue.add(childId);
    }
  }
}
function setPop<T>(set: Set<T>): T {
  let v = nullthrows(set.values().next().value);
  set.delete(v);
  return v;
}

function invertAssetSymbolsMapping(exportSymbolToIdentifierMap) {
  let assetSymbolsInverse = new Map<Symbol, Set<Symbol>>();
  for (let [s, {local}] of exportSymbolToIdentifierMap) {
    let set = assetSymbolsInverse.get(local);

    if (!set) {
      set = new Set();
      assetSymbolsInverse.set(local, set);
    }
    set.add(s);
  }
  return assetSymbolsInverse;
}

function getChangedAssets(assetGraph, changedAssetsPropagation) {
  return new Set(
    [...changedAssetsPropagation].map(id =>
      assetGraph.getNodeIdByContentKey(id),
    ),
  );
}

function sortUsedSymbolsUp(changedDeps) {
  for (let dep of changedDeps) {
    dep.usedSymbolsUp = new Map(
      [...dep.usedSymbolsUp].sort(([a], [b]) => a.localeCompare(b)),
    );
  }
}

function removeNotImportedSymbolsFromDependencies(outgoingDep) {
  outgoingDep.usedSymbolsDown.forEach((_, s) =>
    outgoingDep.usedSymbolsUp.set(s, null),
  );
}

function processOutgoingDependencies(
  assetGraph,
  outgoingDeps,
  assetSymbolsInverse,
  reexportedSymbols,
  reexportedSymbolsSource,
  options,
  assetNode,
) {
  for (let outgoingDep of outgoingDeps) {
    const outgoingDepSymbols = outgoingDep.value.symbols;
    if (!outgoingDepSymbols) continue;

    const isExcluded =
      assetGraph.getNodeIdsConnectedFrom(
        assetGraph.getNodeIdByContentKey(outgoingDep.id),
      ).length === 0;
    if (isExcluded) {
      removeNotImportedSymbolsFromDependencies(outgoingDep);
    }
    if (outgoingDepSymbols.get('*')?.local === '*') {
      processWildcardSymbols(
        outgoingDeps,
        assetNode,
        reexportedSymbols,
        reexportedSymbolsSource,
        outgoingDep,
        options,
      );
    }

    for (let [s, sResolved] of outgoingDep.usedSymbolsUp) {
      if (!outgoingDep.usedSymbolsDown.has(s)) continue;
      const local = outgoingDepSymbols.get(s)?.local;
      if (local == null) continue;
      const reexported = assetSymbolsInverse?.get(local);
      if (reexported != null) {
        reexported.forEach(symbol =>
          processReexportedSymbol(
            reexportedSymbols,
            assetNode,
            symbol,
            sResolved,
            outgoingDep,
            options,
            reexportedSymbolsSource,
          ),
        );
      }
    }
  }
}

function processWildcardSymbols(
  outgoingDep,
  assetNode,
  reexportedSymbols,
  reexportedSymbolsSource,
  outgoingDepNode,
  options,
) {
  outgoingDep.usedSymbolsUp.forEach((sResolved, s) => {
    if (s === 'default') return;
    processReexportedSymbol(
      reexportedSymbols,
      assetNode,
      s,
      sResolved,
      outgoingDepNode,
      options,
      reexportedSymbolsSource,
    );
  });
}

function processReexportedSymbol(
  reexportedSymbols,
  assetNode,
  symbol,
  resolvedSymbol,
  outgoingDep,
  options,
  reexportedSymbolsSource,
) {
  if (reexportedSymbols.has(symbol)) {
    if (!assetNode.usedSymbols.has('')) {
      logFallbackNamespaceInsertion(
        assetNode,
        symbol,
        nullthrows(reexportedSymbolsSource.get(symbol)),
        outgoingDep,
        options,
      );
    }
    assetNode.usedSymbols.add('');
    reexportedSymbols.set(symbol, {asset: assetNode.id, symbol: symbol});
  } else {
    reexportedSymbols.set(symbol, resolvedSymbol);
    reexportedSymbolsSource.set(symbol, outgoingDep);
  }
}

function processIncomingDependencies(
  incomingDeps,
  assetNode,
  assetSymbols,
  reexportedSymbols,
  reexportedSymbolsSource,
  assetGraph,
  options,
  changedDeps,
) {
  const errors = [];
  for (let incomingDep of incomingDeps) {
    const incomingDepUsedSymbolsUpOld = incomingDep.usedSymbolsUp;
    incomingDep.usedSymbolsUp = new Map();
    const incomingDepSymbols = incomingDep.value.symbols;
    if (!incomingDepSymbols) continue;

    const hasNamespaceReexport = incomingDepSymbols.get('*')?.local === '*';
    for (let symbol of incomingDep.usedSymbolsDown) {
      handleSymbolResolution(
        assetNode,
        assetSymbols,
        reexportedSymbols,
        symbol,
        incomingDep,
        assetGraph,
        options,
        errors,
        reexportedSymbolsSource,
        hasNamespaceReexport,
      );
    }

    if (!equalMap(incomingDepUsedSymbolsUpOld, incomingDep.usedSymbolsUp)) {
      changedDeps.add(incomingDep);
      incomingDep.usedSymbolsUpDirtyUp = true;
    }
    excludeUnusedDependencies(incomingDep, assetGraph);
  }
  return errors;
}

function handleSymbolResolution(
  assetNode,
  assetSymbols,
  reexportedSymbols,
  symbol,
  incomingDep,
  assetGraph,
  options,
  errors,
  reexportedSymbolsSource,
  hasNamespaceReexport,
) {
  if (
    assetSymbols == null ||
    assetNode.value.bundleBehavior === BundleBehavior.isolated ||
    assetNode.value.bundleBehavior === BundleBehavior.inline ||
    symbol === '*' ||
    assetNode.usedSymbols.has(symbol)
  ) {
    usedSymbolsUpAmbiguous(incomingDep.usedSymbolsUp, symbol, {
      asset: assetNode.id,
      symbol: symbol,
    });
  } else if (reexportedSymbols.has(symbol)) {
    const reexport = reexportedSymbols.get(symbol);
    const value =
      !assetNode.value.sideEffects && reexport != null
        ? reexport
        : {asset: assetNode.id, symbol: symbol};
    usedSymbolsUpAmbiguous(incomingDep.usedSymbolsUp, symbol, value);
  } else if (!hasNamespaceReexport) {
    const loc = incomingDep.value.symbols?.get(symbol)?.loc;
    const [resolutionNodeId] = assetGraph.getNodeIdsConnectedFrom(
      assetGraph.getNodeIdByContentKey(incomingDep.id),
    );
    const resolution = nullthrows(assetGraph.getNode(resolutionNodeId));
    invariant(
      resolution &&
        (resolution.type === 'asset_group' || resolution.type === 'asset'),
    );

    errors.push({
      message: md`${fromProjectPathRelative(
        resolution.value.filePath,
      )} does not export '${symbol}'`,
      origin: '@atlaspack/core',
      codeFrames: loc
        ? [
            {
              filePath:
                fromProjectPath(options.projectRoot, loc?.filePath) ??
                undefined,
              language: incomingDep.value.sourceAssetType ?? undefined,
              codeHighlights: [convertSourceLocationToHighlight(loc)],
            },
          ]
        : undefined,
    });
  }
}

function usedSymbolsUpAmbiguous(current, symbol, value) {
  const existingValue = current.get(symbol);
  if (
    existingValue !== value &&
    !(
      existingValue?.asset === value.asset &&
      existingValue?.symbol === value.symbol
    )
  ) {
    current.set(symbol, undefined);
  } else {
    current.set(symbol, value);
  }
}

function excludeUnusedDependencies(incomingDep, assetGraph) {
  incomingDep.excluded = false;
  if (
    incomingDep.value.symbols != null &&
    incomingDep.usedSymbolsUp.size === 0
  ) {
    const assetGroups = assetGraph.getNodeIdsConnectedFrom(
      assetGraph.getNodeIdByContentKey(incomingDep.id),
    );
    if (assetGroups.length === 1) {
      const [assetGroupId] = assetGroups;
      const assetGroup = nullthrows(assetGraph.getNode(assetGroupId));
      if (
        assetGroup.type === 'asset_group' &&
        assetGroup.value.sideEffects === false
      ) {
        incomingDep.excluded = true;
      }
    } else {
      invariant(assetGroups.length === 0);
    }
  }
}

function equalMap<K>(
  a: $ReadOnlyMap<K, ?{|asset: ContentKey, symbol: ?Symbol|}>,
  b: $ReadOnlyMap<K, ?{|asset: ContentKey, symbol: ?Symbol|}>,
) {
  if (a.size !== b.size) return false;
  for (let [k, v] of a) {
    if (!b.has(k)) return false;
    let vB = b.get(k);
    if (vB?.asset !== v?.asset || vB?.symbol !== v?.symbol) return false;
  }
  return true;
}
