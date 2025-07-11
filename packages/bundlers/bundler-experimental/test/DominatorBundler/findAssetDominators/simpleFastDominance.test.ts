import assert from 'assert';
import {Graph} from '@atlaspack/graph';
import {simpleFastDominance} from '../../../src/DominatorBundler/findAssetDominators/simpleFastDominance';

const baseGraph = () => {
  const inputGraph = new Graph();
  const root = inputGraph.addNode('root');
  inputGraph.setRootNodeId(root);
  return {inputGraph, root};
};

describe('simpleFastDominance', () => {
  it('it works on a linear graph', () => {
    // digraph g {
    //   root -> a -> b -> c -> d
    // }
    const {inputGraph, root} = baseGraph();
    inputGraph.setRootNodeId(root);

    const a = inputGraph.addNode('a');
    const b = inputGraph.addNode('b');
    const c = inputGraph.addNode('c');
    const d = inputGraph.addNode('d');

    inputGraph.addEdge(root, a);
    inputGraph.addEdge(a, b);
    inputGraph.addEdge(b, c);
    inputGraph.addEdge(c, d);

    const dominators = simpleFastDominance(inputGraph);
    assert.equal(dominators[root], root);
    assert.equal(dominators[a], root);
    assert.equal(dominators[b], a);
    assert.equal(dominators[c], b);
    assert.equal(dominators[d], c);
  });

  it('it works on a tree graph', () => {
    // digraph g {
    //   root -> a;
    //   root -> b;
    //   a -> c;
    //   a -> d;
    //   b -> e;
    // }
    const {inputGraph, root} = baseGraph();

    const a = inputGraph.addNode('a');
    const b = inputGraph.addNode('b');
    const c = inputGraph.addNode('c');
    const d = inputGraph.addNode('d');
    const e = inputGraph.addNode('e');

    inputGraph.addEdge(root, a);
    inputGraph.addEdge(root, b);
    inputGraph.addEdge(a, c);
    inputGraph.addEdge(a, d);
    inputGraph.addEdge(b, e);

    const dominators = simpleFastDominance(inputGraph);
    assert.equal(dominators[root], root);
    assert.equal(dominators[a], root);
    assert.equal(dominators[b], root);
    assert.equal(dominators[c], a);
    assert.equal(dominators[d], a);
    assert.equal(dominators[e], b);
  });

  it('it works on simple graph with multiple paths', () => {
    // digraph g {
    //   root -> a;
    //   a -> b;
    //   b -> c;
    //   a -> c;
    // }
    const {inputGraph, root} = baseGraph();

    const a = inputGraph.addNode('a');
    const b = inputGraph.addNode('b');
    const c = inputGraph.addNode('c');

    inputGraph.addEdge(root, a);
    inputGraph.addEdge(a, b);
    inputGraph.addEdge(b, c);
    inputGraph.addEdge(a, c);

    const dominators = simpleFastDominance(inputGraph);
    assert.equal(dominators[root], root);
    assert.equal(dominators[a], root);
    assert.equal(dominators[b], a);
    assert.equal(dominators[c], a);
  });

  it('it works on a graph with multiple paths to nodes', () => {
    // digraph g {
    //   root -> a;
    //   root -> b;
    //   a -> c;
    //   a -> d;
    //   b -> d;
    //   d -> c;
    // }
    const {inputGraph, root} = baseGraph();

    const a = inputGraph.addNode('a');
    const b = inputGraph.addNode('b');
    const c = inputGraph.addNode('c');
    const d = inputGraph.addNode('d');

    inputGraph.addEdge(root, a);
    inputGraph.addEdge(root, b);
    inputGraph.addEdge(a, c);
    inputGraph.addEdge(a, d);
    inputGraph.addEdge(b, d);
    inputGraph.addEdge(d, c);

    const dominators = simpleFastDominance(inputGraph);
    assert.equal(dominators[root], root);
    assert.equal(dominators[a], root);
    assert.equal(dominators[b], root);
    assert.equal(dominators[c], root);
    assert.equal(dominators[d], root);
  });
});
