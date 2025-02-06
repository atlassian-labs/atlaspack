// @flow strict-local

import {Graph} from '@atlaspack/graph';
import assert from 'assert';
import {findStronglyConnectedComponents} from '../../../src/DominatorBundler/cycleBreaker/findStronglyConnectedComponents';

describe('findStronglyConnectedComponents', () => {
  it('should find strongly connected components', () => {
    /*
      digraph graph {
        root -> a;
        a -> b;
        b -> a;
        c;
      }
     */
    const graph = new Graph();
    const root = graph.addNode('root');
    graph.setRootNodeId(root);
    const a = graph.addNode('a');
    const b = graph.addNode('b');
    const c = graph.addNode('c');

    graph.addEdge(root, a);
    graph.addEdge(a, b);
    graph.addEdge(b, a);

    const result = findStronglyConnectedComponents(graph);

    assert.deepStrictEqual(result, [[b, a], [root], [c]]);
  });

  it('works over an empty graph with a root', () => {
    /*
      digraph graph {
        root;
      }
     */
    const graph = new Graph();
    const root = graph.addNode('root');
    graph.setRootNodeId(root);

    const result = findStronglyConnectedComponents(graph);
    assert.deepStrictEqual(result, [[root]]);
  });

  it('works over an empty graph', () => {
    /*
      digraph graph {
      }
     */
    const graph = new Graph();

    const result = findStronglyConnectedComponents(graph);
    assert.deepStrictEqual(result, []);
  });

  it('returns all nodes if there are no cycles', () => {
    /*
      digraph graph {
        root -> a;
        a -> b;
        b -> c;
      }
     */
    const graph = new Graph();
    const root = graph.addNode('root');
    graph.setRootNodeId(root);
    const a = graph.addNode('a');
    const b = graph.addNode('b');
    const c = graph.addNode('c');

    graph.addEdge(root, a);
    graph.addEdge(a, b);
    graph.addEdge(b, c);

    const result = findStronglyConnectedComponents(graph);

    assert.deepStrictEqual(result, [[c], [b], [a], [root]]);
  });
});
