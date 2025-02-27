// @flow strict-local

import assert from 'assert';
import ContentGraph from '../src/ContentGraph';

describe('ContentGraph', () => {
  it('should addNodeByContentKey if no node exists with the content key', () => {
    let graph = new ContentGraph();

    const node = {};

    const nodeId1 = graph.addNodeByContentKey('contentKey', node);

    assert.deepEqual(graph.getNode(nodeId1), node);
    assert(graph.hasContentKey('contentKey'));
    assert.deepEqual(graph.getNodeByContentKey('contentKey'), node);
  });

  it('should throw if a node with the content key already exists', () => {
    let graph = new ContentGraph();

    graph.addNodeByContentKey('contentKey', {});

    assert.throws(() => {
      graph.addNodeByContentKey('contentKey', {});
    }, /already has content key/);
  });

  it('should remove the content key from graph when node is removed', () => {
    let graph = new ContentGraph();

    const node1 = {};
    const nodeId1 = graph.addNodeByContentKey('contentKey', node1);

    assert.equal(graph.getNode(nodeId1), node1);
    assert(graph.hasContentKey('contentKey'));

    graph.removeNode(nodeId1);

    assert(!graph.hasContentKey('contentKey'));
  });

  describe('getContentKeyByNodeId', () => {
    it('returns the content key for a node', () => {
      const graph = new ContentGraph();

      const node1 = graph.addNodeByContentKey('node1', {});
      assert.equal(graph.getContentKeyByNodeId(node1), 'node1');
      const node2 = graph.addNodeByContentKey('node2', {});
      assert.equal(graph.getContentKeyByNodeId(node2), 'node2');
    });

    it('throws if the node does not have a content key', () => {
      const graph = new ContentGraph();

      const node1 = graph.addNode({});
      assert.throws(() => graph.getContentKeyByNodeId(node1));
    });
  });
});
