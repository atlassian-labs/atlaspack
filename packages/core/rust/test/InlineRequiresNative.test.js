// @flow strict-local

import assert from 'assert';
import {runInlineRequiresOptimizer} from '..';

describe('runInlineRequiresOptimizer', () => {
  it('replaces inline code on source', () => {
    const result = runInlineRequiresOptimizer({
      inputCode: `
const fs = require('fs');

function main() {
    return fs.readFile('./something');
}`,
      assetsToIgnore: [],
      sourceMaps: true,
    });

    assert.equal(
      result.outputCode,
      `
const fs;
function main() {
    return require('fs').readFile('./something');
}
`.trimStart(),
    );
    // $FlowFixMe
    const sourceMap = JSON.parse(result.sourceMap);
    assert.ok(sourceMap);
  });
});
