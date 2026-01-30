import assert from 'assert';
import path from 'path';
import { bundle, describe, it, outputFS, distDir, disableV3 } from '@atlaspack/test-utils';

describe('namer', function () {
  disableV3();

  it('should determine correct entry root when building a directory', async function () {
    await bundle(path.join(__dirname, 'integration/namer-dir'));

    assert(await outputFS.exists(path.join(distDir, 'index.html')));
    assert(await outputFS.exists(path.join(distDir, 'nested/other.html')));
  });
});
