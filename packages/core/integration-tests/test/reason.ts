import assert from 'assert';
import path from 'path';
import {bundle, run} from '@atlaspack/test-utils';

describe.skip('reason', function () {
  it('should produce a bundle', async function () {
    const b = await bundle(
      path.join(__dirname, '/integration/reason/index.js'),
    );

    assert.equal(b.assets.size, 2);
    assert.equal(b.childBundles.size, 1);

    const output = await run(b);
    assert.equal(typeof output, 'function');
    assert.equal(output(), 3);
  });
});
