// @flow strict-local

import assert from 'assert';
import {EdgeContentGraph} from '../../src/DominatorBundler/EdgeContentGraph';

describe('EdgeContentGraph', () => {
  it('should add and get edge weights', () => {
    const graph = new EdgeContentGraph<string, number>();
    const a = graph.addNode('a');
    const b = graph.addNode('b');

    graph.addWeightedEdge(a, b, 1, 10);
    assert.equal(graph.getEdgeWeight(a, b), 10);
  });
});
