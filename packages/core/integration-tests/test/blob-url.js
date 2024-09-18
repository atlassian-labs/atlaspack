// @flow strict-local

import assert from 'assert';
import {join} from 'path';
import {
  bundle,
  describe,
  distDir,
  it,
  outputFS,
  run,
} from '@atlaspack/test-utils';

class Blob {
  data;
  constructor(data) {
    this.data = data;
  }
}

const URL = {
  createObjectURL(blob) {
    assert(blob instanceof Blob);
    return `data:application/javascript,${encodeURIComponent(blob.data)}`;
  },
};

describe('blob-url:', () => {
  it('inlines and compiles content as a blob url', async () => {
    let b = await bundle(join(__dirname, '../data/integration/blob-url/index.js'));

    let created = [];

    class Worker {
      constructor(src) {
        created.push(src);
      }
      postMessage() {}
    }

    await run(b, {
      Worker,
      Blob,
      URL,
    });

    assert.equal(created.length, 1);
    assert(created[0].startsWith('data:application/javascript,'));

    let content = await outputFS.readFile(join(distDir, 'index.js'), 'utf8');

    assert(content.includes('new Worker(require('));
    assert(
      content.includes(
        'module.exports = URL.createObjectURL(new Blob(["// modules are defined as an array\\n',
      ),
    );
    assert(
      content.includes(
        'self.postMessage(\\"this should appear in the bundle\\\\n\\")',
      ),
    );
  });

  it('inlines, compiles, and minifies content as a blob url', async () => {
    let b = await bundle(join(__dirname, '../data/integration/blob-url/index.js'), {
      defaultTargetOptions: {
        shouldOptimize: true,
      },
    });

    let created = [];

    class Worker {
      constructor(src) {
        created.push(src);
      }
      postMessage() {}
    }

    await run(b, {
      Worker,
      Blob,
      URL,
    });

    assert.equal(created.length, 1);
    assert(created[0].startsWith('data:application/javascript,'));

    let content = await outputFS.readFile(join(distDir, 'index.js'), 'utf8');

    assert(content.includes('new Worker('));
    assert(
      content.includes(".exports=URL.createObjectURL(new Blob(['!function("),
    );
    assert(
      content.includes(
        'self.postMessage("this should appear in the bundle\\\\n")',
      ),
    );
  });
});
