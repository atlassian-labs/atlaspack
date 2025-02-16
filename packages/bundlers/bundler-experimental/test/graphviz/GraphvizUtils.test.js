// @flow strict-local

import {dotFromBundleGraph, runDotForTest} from './GraphvizUtils';

import assert from 'assert';
import fs from 'fs';
import child_process from 'child_process';
import sinon from 'sinon';

describe('runDotForTest', () => {
  afterEach(() => {
    sinon.restore();
  });

  it('writes the dot file to disk', () => {
    const writeFileSync = sinon.stub(fs, 'writeFileSync');
    const mkdirSync = sinon.stub(fs, 'mkdirSync');
    const execSync = sinon.stub(child_process, 'execSync');

    const __dirname = '/path/to/dir';
    const __filename = 'GraphvizUtils.tests.js';
    const name = 'test 1234';
    const label = 'label';
    const dot = 'digraph { a -> b; }';

    runDotForTest(__dirname, __filename, name, label, dot);

    assert.equal(
      mkdirSync.withArgs(
        '/path/to/dir/__graphs__/GraphvizUtils.tests.js - test 1234/dot',
        {recursive: true},
      ).callCount,
      1,
    );

    assert.equal(
      writeFileSync.withArgs(
        '/path/to/dir/__graphs__/GraphvizUtils.tests.js - test 1234/dot/label.dot',
        'digraph { a -> b; }',
      ).callCount,
      1,
    );

    assert.equal(
      execSync.withArgs(
        'dot -Tsvg -o "/path/to/dir/__graphs__/GraphvizUtils.tests.js - test 1234/svg/label.dot.svg" "/path/to/dir/__graphs__/GraphvizUtils.tests.js - test 1234/dot/label.dot"',
      ).callCount,
      1,
    );
  });
});

describe('dotFromBundleGraph', () => {
  it('creates a dot string from a bundle graph', () => {
    // $FlowFixMe
    const mockBundleGraph: any = {
      getBundles: () => [
        {
          id: '1',
          traverseAssets: (cb) => {
            cb({filePath: '/path/to/file1.js'});
            cb({filePath: '/path/to/file2.js'});
          },
        },
        {
          id: '2',
          traverseAssets: (cb) => {
            cb({filePath: '/path/to/file3.js'});
            cb({filePath: '/path/to/file4.js'});
          },
        },
      ],
    };

    const dot = dotFromBundleGraph('/path/to', mockBundleGraph);
    assert.equal(
      dot,
      `
digraph bundle_graph {
  labelloc="t";
  label="Bundle graph";

  subgraph cluster_1 {
    label = "Bundle 1";
    "file1.js";
    "file2.js";
  }
  subgraph cluster_2 {
    label = "Bundle 2";
    "file3.js";
    "file4.js";
  }
}
      `.trim(),
    );
  });
});
