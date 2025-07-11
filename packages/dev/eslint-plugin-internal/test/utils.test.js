'use strict';

const assert = require('assert');
const path = require('path');
const {parse} = require('@babel/eslint-parser');
const readPkgUp = require('read-pkg-up');

const {
  isStaticRequire,
  isStaticResolve,
  relativePathForRequire,
} = require('../src/utils');

const pkgInfo = readPkgUp.sync({cwd: __dirname});
const pkgPath = pkgInfo.path;
const pkgName = pkgInfo.pkg.name;

describe('utils', () => {
  let opts = {
    babelOptions: {filename: 'test.js'},
  };

  describe('isRequire', () => {
    it('identifies requires', () => {
      const parsed = parse("require('@atlaspack/core')", opts);
      const expr = getFirstExpression(parsed, opts);
      assert.equal(isStaticRequire(expr), true);
    });

    it("doesn't handle dynamic requires", () => {
      const parsed = parse('require(dynamic)', opts);
      const expr = getFirstExpression(parsed, opts);
      assert.equal(isStaticRequire(expr), false);
    });
  });

  describe('isResolve', () => {
    it('identifies built-in require.resolve', () => {
      const parsed = parse("require.resolve('@atlaspack/core')", opts);
      const expr = getFirstExpression(parsed, opts);
      assert.equal(isStaticResolve(expr), true);
    });
  });

  describe('relativePathForRequire', () => {
    it('behaves identically as path.relative on unix', () => {
      let sep = path.sep;
      path.sep = '/';
      assert.equal(
        relativePathForRequire({
          origin: __filename,
          request: '@atlaspack/eslint-plugin-internal/',
          pkgName,
          pkgPath,
        }),
        '../',
      );
      path.sep = sep;
    });

    it('uses / to separate paths even when path.sep is not /', () => {
      let sep = path.sep;
      path.sep = '\\';
      assert.equal(
        relativePathForRequire({
          origin: __filename,
          request: '@atlaspack/eslint-plugin-internal/',
          pkgName,
          pkgPath,
        }),
        '../',
      );
      path.sep = sep;
    });

    it('leaves absolute paths alone', () => {
      assert.equal(
        relativePathForRequire({
          origin: __filename,
          request: '/a/b',
          pkgName,
          pkgPath,
        }),
        '/a/b',
      );
    });

    it('prepends ./ to peer paths', () => {
      assert.equal(
        relativePathForRequire({
          origin: __filename,
          request: '@atlaspack/eslint-plugin-internal/test/baz',
          pkgName,
          pkgPath,
        }),
        './baz',
      );
    });
  });
});

function getFirstExpression(program) {
  return program.body[0].expression;
}
