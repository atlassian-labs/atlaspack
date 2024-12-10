// @flow
import assert from 'assert';
import Module from 'module';
import path from 'path';
import fs from 'fs';
import {
  bundle,
  describe,
  it,
  run,
  assertBundles,
  inputFS,
} from '@atlaspack/test-utils';

const ZIPFS = `${path.sep}zipfs`;

describe.v2('pnp', function () {
  it('should defer to the pnp resolution when needed', async function () {
    let dir = path.join(__dirname, 'integration/pnp-require');

    let origPnpVersion = process.versions.pnp;
    process.versions.pnp = `${42}`;

    // $FlowFixMe
    let origModuleResolveFilename = Module._resolveFilename;
    // $FlowFixMe
    Module.findPnpApi = () => require(path.join(dir, '.pnp.js'));
    // $FlowFixMe
    Module._resolveFilename = (name, ...args) =>
      name === 'pnpapi'
        ? path.join(dir, '.pnp.js')
        : origModuleResolveFilename(name, ...args);

    // $FlowFixMe
    let origReadFileSync = inputFS.readFileSync;
    // $FlowFixMe
    inputFS.readFileSync = (p, ...args) => {
      return origReadFileSync.call(inputFS, p.replace(ZIPFS, ''), ...args);
    };

    let origRealpathSync = fs.realpathSync;
    inputFS.realpathSync = (p, ...args) => {
      return origRealpathSync.call(inputFS, p.replace(ZIPFS, ''), ...args);
    };

    let origStatSync = inputFS.statSync;
    inputFS.statSync = (p, ...args) => {
      return origStatSync.call(inputFS, p.replace(ZIPFS, ''), ...args);
    };

    try {
      let b = await bundle(path.join(dir, 'index.js'));

      assertBundles(b, [
        {
          name: 'index.js',
          assets: ['index.js', 'local.js', 'index.js'],
        },
      ]);

      let output = await run(b);
      assert.equal(output(), 3);
    } finally {
      process.versions.pnp = origPnpVersion;
      // $FlowFixMe
      Module._resolveFilename = origModuleResolveFilename;
      // $FlowFixMe
      inputFS.readFileSync = origReadFileSync;
      inputFS.statSync = origStatSync;
      inputFS.realpathSync = origRealpathSync;
    }
  });

  it('should support importing Node builtin modules from npm when requested', async function () {
    let dir = path.join(__dirname, 'integration/pnp-builtin');

    let origPnpVersion = process.versions.pnp;
    process.versions.pnp = `${42}`;

    // $FlowFixMe
    let origModuleResolveFilename = Module._resolveFilename;
    // $FlowFixMe
    Module.findPnpApi = () => require(path.join(dir, '.pnp.js'));
    // $FlowFixMe
    Module._resolveFilename = (name, ...args) =>
      name === 'pnpapi'
        ? path.join(dir, '.pnp.js')
        : origModuleResolveFilename(name, ...args);

    try {
      let b = await bundle(path.join(dir, 'index.js'));

      assertBundles(b, [
        {
          name: 'index.js',
          assets: ['index.js', 'local.js', 'index.js'],
        },
      ]);

      let output = await run(b);
      assert.equal(output(), 3);
    } finally {
      process.versions.pnp = origPnpVersion;
      // $FlowFixMe
      Module._resolveFilename = origModuleResolveFilename;
    }
  });
});
