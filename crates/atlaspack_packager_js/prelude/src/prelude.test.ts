import assert from 'assert';
import fs from 'fs';
import path from 'path';
import type { AtlaspackPrelude } from './prelude';

describe('prelude', () => {
  // Assumption is that Rust build has been run first to create the prelude
  let atlaspack: AtlaspackPrelude;
  before(() => {
    const preludeCode = fs.readFileSync(path.join(__dirname, '../lib/prelude.dev.js'), 'utf8');
    atlaspack = eval(preludeCode);
  });
  beforeEach(() => {
    atlaspack.__reset?.();
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
  it('should only execute a module factory once', () => {
    atlaspack.define('test', (require, module, exports, global) => {
      let count = 0;
      exports.test = () => {
        count += 1;
        return count;
      };
    });
    type testModule = { test: () => number };
    assert((atlaspack.require('test') as testModule).test() === 1);
    assert((atlaspack.require('test') as testModule).test() === 2); // would be 1 if the factory was executed again
  });
})
