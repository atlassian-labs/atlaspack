const deepClone = require('deep-clone');
const diff = require('./diff');

function filterNode(node) {
  let clone = deepClone(node);

  // Clean up anything you don't want to see in the diff
  delete clone.id;
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

  return clone;
}

function assetGraphDiff(jsAssetGraph, rustAssetGraph) {
  const getNodes = graph => {
    let nodes = {};

    graph.traverse(nodeId => {
      let node = graph.getNode(nodeId);

      if (node.type === 'dependency') {
        nodes[`dep:${node.value.sourcePath}:${node.value.specifier}`] =
          filterNode(node);
      } else if (node.type === 'asset') {
        nodes[`asset:${node.value.filePath}`] = filterNode(node);
      }
    });

    return nodes;
  };

  const jsNodes = getNodes(jsAssetGraph);
  const rustNodes = getNodes(rustAssetGraph);

  for (const [key, jsNode] of Object.entries(jsNodes)) {
    let rustNode = rustNodes[key];

    if (!rustNode) {
      // eslint-disable-next-line
      console.log('Missing', key);
      continue;
    }

    // eslint-disable-next-line
    console.log(key);
    diff(jsNode, rustNode);
  }
}

module.exports = assetGraphDiff;
