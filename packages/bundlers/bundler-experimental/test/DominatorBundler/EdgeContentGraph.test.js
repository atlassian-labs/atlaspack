// @flow strict-local

import assert from 'assert';
import {EdgeContentGraph} from '../../src/DominatorBundler/EdgeContentGraph';

describe('EdgeContentGraph', () => {
  describe('addWeightedEdge', () => {
    it('creates an edge between two nodes', () => {
      const graph = new EdgeContentGraph<string, number>();
      const a = graph.addNode('a');
      const b = graph.addNode('b');
      const edgeType = 1;
      const weight = 10;
      graph.addWeightedEdge(a, b, edgeType, weight);
      const ids = graph.getNodeIdsConnectedFrom(a);
      assert.deepEqual(ids, [b]);
    });

    it('should add and get edge weights', () => {
      const graph = new EdgeContentGraph<string, number>();
      const a = graph.addNode('a');
      const b = graph.addNode('b');
      const c = graph.addNode('b');

      const edgeType = 1;
      const weight1 = 10;
      const weight2 = 20;

      graph.addWeightedEdge(a, b, edgeType, weight1);
      graph.addWeightedEdge(b, c, edgeType, weight2);

      assert.equal(graph.getEdgeWeight(a, b), weight1);
      assert.equal(graph.getEdgeWeight(b, c), weight2);
    });
  });

  describe('clone', () => {
    it('clones the graph', () => {
      const graph = new EdgeContentGraph<string, number>();
      const a = graph.addNode('a');
      const b = graph.addNode('b');
      const c = graph.addNode('c');

      const edgeType = 1;
      const weight1 = 10;
      const weight2 = 20;

      graph.addWeightedEdge(a, b, edgeType, weight1);
      graph.addWeightedEdge(b, c, edgeType, weight2);

      const clonedGraph = graph.clone();
      assert.deepEqual(clonedGraph.nodes, graph.nodes);
      assert.deepEqual(clonedGraph.getAllEdges(), graph.getAllEdges());
      assert.equal(clonedGraph.getEdgeWeight(a, b), weight1);
      assert.equal(clonedGraph.getEdgeWeight(b, c), weight2);
    });
  });
});
