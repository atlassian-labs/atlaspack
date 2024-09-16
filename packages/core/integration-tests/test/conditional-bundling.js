import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  fsFixture,
  it,
  run,
  overlayFS,
  removeDistDirectory,
} from '@atlaspack/test-utils';

describe('conditional bundling', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it(`when disabled, should treat importCond as a sync import`, async function () {
    const dir = path.join(__dirname, 'disabled-import-cond');
    overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      index.js:
        globalThis.__MOD_COND = { 'cond': true };

        const result = importCond('cond', './a.js', './b.js');

        export default result;

      a.js:
        export default 'module-a';

      b.js:
        export default 'module-b';
    `;

    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      featureFlags: {conditionalBundlingApi: false},
    });

    let output = await run(b);
    assert.deepEqual(output?.default, 'module-a');
  });
});
