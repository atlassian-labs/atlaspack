// @flow strict-local

import {describe, it, before} from 'node:test';
import assert from 'assert';
import {MemoryFS} from '@atlaspack/fs';
import {workerFarm} from '@atlaspack/test-utils';
import {asset, fixtureFromGraph, dotFromGraph} from './fixtureFromGraph';

describe('fixtureFromGraph', async () => {
  await before(async function () {
    // this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  await it('can create fixtures for single files', async () => {
    const fs = new MemoryFS(workerFarm);
    await fixtureFromGraph('dir', fs, [
      asset('file1.js'),
      asset('file2.js'),
      asset('file3.js'),
    ]);

    assert.deepEqual(await fs.readdir('dir'), [
      'file1.js',
      'file2.js',
      'file3.js',
    ]);
    assert.equal(
      await fs.readFile('dir/file1.js', 'utf8'),
      'export default function run() { return [] }',
    );
  });

  await it('will create files with imports between themselves', async () => {
    const fs = new MemoryFS(workerFarm);
    await fixtureFromGraph('dir', fs, [
      asset('file1.js', ['file2.js', 'file3.js']),
      asset('file2.js'),
      asset('file3.js'),
    ]);

    assert.deepEqual(await fs.readdir('dir'), [
      'file1.js',
      'file2.js',
      'file3.js',
    ]);
    assert.equal(
      await fs.readFile('dir/file1.js', 'utf8'),
      `
import d0 from './file2.js';
import d1 from './file3.js';
export default function run() { return [d0, d1] }
      `.trim(),
    );
  });

  await it('will create files with async imports between themselves', async () => {
    const fs = new MemoryFS(workerFarm);
    await fixtureFromGraph('dir', fs, [
      asset('file1.js', ['file2.js', {to: 'file3.js', type: 'async'}]),
      asset('file2.js'),
      asset('file3.js'),
    ]);

    assert.deepEqual(await fs.readdir('dir'), [
      'file1.js',
      'file2.js',
      'file3.js',
    ]);
    assert.equal(
      await fs.readFile('dir/file1.js', 'utf8'),
      `
import d0 from './file2.js';
export default function run() { return [d0, import('./file3.js')] }
      `.trim(),
    );
  });

  describe('dotFromGraph', () => {
    it('creates a dot string from a graph', () => {
      const graph = [
        asset('file1.js', ['file2.js', 'file3.js']),
        asset('file2.js'),
        asset('file3.js'),
      ];
      const dot = dotFromGraph(graph);

      assert.equal(
        dot,
        `
digraph assets {
  labelloc="t";
  label="Assets";

  "file1.js";
  "file2.js";
  "file3.js";

  "file1.js" -> "file2.js";
  "file1.js" -> "file3.js";
}
        `.trim(),
      );
    });
  });
});
