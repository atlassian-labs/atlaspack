// @flow
import assert from 'assert';
import path from 'path';
import {bundle, describe, it, outputFS} from '@atlaspack/test-utils';

describe.v2('tailwind', function () {
  it('should support tailwind from SCSS', async function () {
    let fixture = path.join(__dirname, '/integration/tailwind-scss');
    let b = await bundle(path.join(fixture, 'index.html'));

    let cssBundle = b.getBundles().find((b) => b.type === 'css');
    if (!cssBundle) return assert.fail();

    let css = await outputFS.readFile(cssBundle.filePath, 'utf8');
    assert(css.includes('.p-2'));
    assert(!css.includes('.m-2'));
  });
});
