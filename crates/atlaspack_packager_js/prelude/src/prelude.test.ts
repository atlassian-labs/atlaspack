import assert from 'assert';
import fs from 'fs';
import path from 'path';
import type { AtlaspackPrelude } from './prelude';
import { execSync } from 'child_process';
import { rolldown, type RolldownOptions } from 'rolldown';
import { preludeConfig } from '../rolldown.config';

async function getPreludeCode(mode: 'dev' | 'prod') {
  const config: RolldownOptions = {
    ...preludeConfig(mode),
    cwd: path.join(__dirname, '../')
  };
  const devPrelude = await rolldown(config);
  if (!config.output || Array.isArray(config.output)) {
    throw new Error('Invalid output config');
  }
  return (await devPrelude.generate(config.output)).output[0].code;
}

describe('prelude', () => {
  // Assumption is that Rust build has been run first to create the prelude
  let atlaspack: AtlaspackPrelude;
  before(async () => {
    // Build the prelude
    const preludeCode = await getPreludeCode('dev');
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
