import assert from 'assert';
import {runInlineRequiresOptimizer} from '..';

describe('runInlineRequiresOptimizer', () => {
  it('replaces inline code on source', () => {
    const result = runInlineRequiresOptimizer({
      code: `
const fs = require('fs');

function main() {
    return fs.readFile('./something');
}`,
      ignoreModuleIds: [],
      sourceMaps: true,
    });

    assert.equal(
      result.code,
      `
const fs;
function main() {
    return require('fs').readFile('./something');
}
`.trimStart(),
    );
    // @ts-expect-error - TS2345 - Argument of type 'string | undefined' is not assignable to parameter of type 'string'.
    const sourceMap = JSON.parse(result.sourceMap);
    assert.ok(sourceMap);
  });
});
