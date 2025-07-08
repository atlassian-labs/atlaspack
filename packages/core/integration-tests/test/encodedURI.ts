import assert from 'assert';
import {join} from 'path';
import {bundle, describe, it, outputFS, distDir} from '@atlaspack/test-utils';

describe('encodedURI', function () {
  it('should support bundling files which names in encoded URI', async function () {
    await bundle(join(__dirname, '/integration/encodedURI/index.html'));

    const files = await outputFS.readdir(distDir);
    const html = await outputFS.readFile(join(distDir, 'index.html'));
    for (const file of files) {
      if (file !== 'index.html') {
        assert(html.includes(file));
      }
    }
    assert(!!files.find((f) => f.startsWith('日本語')));
  });
});
