import assert from 'assert';
import path from 'path';
import {bundle, describe, it, run} from '@atlaspack/test-utils';

describe.v2('mdx', function () {
  it('should support bundling MDX', async function () {
    const b = await bundle(path.join(__dirname, '/integration/mdx/index.mdx'));

    const output = await run(b);
    assert.equal(typeof output.default, 'function');
    assert(output.default.isMDXComponent);
  });

  it('should support bundling MDX with React 17', async function () {
    const b = await bundle(
      path.join(__dirname, '/integration/mdx-react-17/index.mdx'),
    );

    const output = await run(b);
    assert.equal(typeof output.default, 'function');
    assert(output.default.isMDXComponent);
  });
});
