import assert from 'assert';
import './prelude.ts';

describe('prelude', () => {
  let atlaspack: AtlaspackPrelude = global.atlaspack_ATLASPACK_PRELUDE_HASH;
  beforeEach(() => {
    atlaspack.__reset();
  });
  it('should be able to define and require a module', () => {
    assert(typeof atlaspack.require === 'function');

    atlaspack.define('test', (require, module, exports, global) => {
      exports.test = 'hello!';
    });

    assert(atlaspack.require('test').test === 'hello!');
  });
  it('should throw an error if the module is not found', () => {
    assert.throws(() => atlaspack.require('test'), /Cannot find module 'test'/);
  });
})
