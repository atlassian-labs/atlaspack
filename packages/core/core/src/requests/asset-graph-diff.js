/* eslint-disable */
const deepClone = require('rfdc/default');
const diff = require('jest-diff').diff;

function filterNode(node) {
  let clone = deepClone(node);

  // Clean up anything you don't want to see in the diff
  // delete clone.id;
  delete clone.value.id;
  delete clone.value.meta.id;
  delete clone.value.sourceAssetId;
  delete clone.value.env.id;
  delete clone.value.isEsm;
  delete clone.value.shouldWrap;
  delete clone.value.contentKey;
  delete clone.value.placeholder;
  delete clone.value.code;
  delete clone.value.hasCjsExports;
  delete clone.value.staticExports;
  delete clone.value.isConstantModule;
  delete clone.value.hasNodeReplacements;
  delete clone.value.stats;
  delete clone.value.astKey;
  delete clone.value.astGenerator;
  delete clone.value.dependencies;

  return clone;
}

function compactDeep(obj) {
  if (obj instanceof Map) {
    const copy = {};
    Array.from(obj.entries()).forEach(([k, v]) => {
      if (v != null) {
        copy[k] = compactDeep(v);
      }
    });
    return copy;
  } else if (Array.isArray(obj)) {
    return obj.map(v => compactDeep(v));
  } else if (typeof obj === 'object') {
    const copy = {};
    Object.entries(obj ?? {}).forEach(([key, value]) => {
      // We won't be exposing this ; this is used for persistence on js side
      if (key === 'mapKey') {
        return;
      }
      // We won't be exposing this
      if (key === 'correspondingRequest') {
        return;
      }
      // Equivalent false == null
      if (key === 'isWeak' && value === false) {
        return;
      }

      if (value != null) {
        copy[key] = compactDeep(value);
      }
    });
    return copy;
  } else if (obj != null) {
    return obj;
  }
}

function assetGraphDiff(jsAssetGraph, rustAssetGraph) {
  const getNodes = graph => {
    let nodes = {};

    graph.traverse(nodeId => {
      let node = graph.getNode(nodeId);

      if (node.type === 'dependency') {
        let sourcePath = node.value.sourcePath ?? 'entry';
        nodes[`dep:${sourcePath}:${node.value.specifier}`] = filterNode(node);
      } else if (node.type === 'asset') {
        nodes[`asset:${node.value.filePath}`] = filterNode(node);
      }
    });

    return nodes;
  };

  const jsNodes = getNodes(jsAssetGraph);
  const rustNodes = getNodes(rustAssetGraph);

  const all = new Set([...Object.keys(jsNodes), ...Object.keys(rustNodes)]);
  const missing = [];
  const extra = [];

  for (const key of all.keys()) {
    if (
      !(
        process.env.NATIVE_COMPARE === 'true' ||
        key.includes(process.env.NATIVE_COMPARE)
      )
    ) {
      continue;
    }
    let jsNode = jsNodes[key];
    let rustNode = rustNodes[key];

    if (!rustNode) {
      missing.push(key);
      continue;
    }
    if (!jsNode) {
      extra.push(key);
      continue;
    }

    console.log(key);
    console.log(diff(compactDeep(rustNode), compactDeep(jsNode)));
  }

  console.log('Missing', missing);
  console.log('Extra', extra);
}

module.exports = assetGraphDiff;
