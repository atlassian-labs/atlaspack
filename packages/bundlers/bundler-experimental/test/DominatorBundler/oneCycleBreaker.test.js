// @flow strict-local

import {ContentGraph} from '@atlaspack/graph';
import {findStronglyConnectedComponents} from '../../src/DominatorBundler/oneCycleBreaker';
import assert from 'assert';

describe('oneCycleBreaker', () => {
  describe('findStronglyConnectedComponents', () => {
    it('should find strongly connected components', () => {
      const graph = new ContentGraph();
      graph.addNodeByContentKey('root', 'root');
      graph.setRootNodeId(0);
      graph.addNodeByContentKey('a', 'a');
      graph.addNodeByContentKey('b', 'b');
      graph.addNodeByContentKey('c', 'c');

      graph.addEdge(0, 1);
      graph.addEdge(1, 2);
      graph.addEdge(2, 1);

      const result = findStronglyConnectedComponents(graph);

      assert.deepStrictEqual(result, [[2, 1], [0]]);
    });
  });
});
