// @flow
import assert from 'assert';
import {join} from 'path';
import {bundle, describe, it, outputFS, distDir} from '@atlaspack/test-utils';

describe('encodedURI', function () {
  it('should support bundling files which names in encoded URI', async function () {
    await bundle(join(__dirname, '/integration/encodedURI/index.html'));

    let files = await outputFS.readdir(distDir);
    let html = await outputFS.readFile(join(distDir, 'index.html'));
    for (let file of files) {
      if (file !== 'index.html') {
        assert(html.includes(file));
      }
    }
    assert(!!files.find((f) => f.startsWith('日本語')));
  });
});
