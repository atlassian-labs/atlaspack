// @flow strict-local

import assert from 'assert';
import UncommittedAsset from '../src/UncommittedAsset';
import {
  createAsset as _createAsset,
  type AssetOptions,
} from '../src/assetUtils';
import {createEnvironment} from '../src/Environment';
import {DEFAULT_OPTIONS} from './test-utils';
import {toProjectPath} from '../src/projectPath';

function createAsset(opts: AssetOptions) {
  return _createAsset('/', opts);
}

const stats = {time: 0, size: 0};

describe('InternalAsset', () => {
  it('only includes connected files once per filePath', () => {
    let asset = new UncommittedAsset({
      value: createAsset({
        filePath: toProjectPath('/', '/foo/asset.js'),
        code: null,
        env: createEnvironment(),
        stats,
        type: 'js',
        isSource: true,
      }),
      options: DEFAULT_OPTIONS,
    });
    asset.invalidateOnFileChange(toProjectPath('/', '/foo/file'));
    asset.invalidateOnFileChange(toProjectPath('/', '/foo/file'));
    assert.deepEqual(
      asset.invalidations.invalidateOnFileChange,
      new Set(['foo/file']),
    );
  });

  it('only includes dependencies once per id', () => {
    let asset = new UncommittedAsset({
      value: createAsset({
        filePath: toProjectPath('/', '/foo/asset.js'),
        code: null,
        env: createEnvironment(),
        stats,
        type: 'js',
        isSource: true,
      }),
      options: DEFAULT_OPTIONS,
    });

    asset.addDependency({specifier: './foo', specifierType: 'esm'});
    asset.addDependency({specifier: './foo', specifierType: 'esm'});
    let dependencies = asset.getDependencies();
    assert(dependencies.length === 1);
    assert(dependencies[0].specifier === './foo');
  });

  it('includes different dependencies if their id differs', () => {
    let asset = new UncommittedAsset({
      value: createAsset({
        filePath: toProjectPath('/', '/foo/asset.js'),
        code: null,
        env: createEnvironment(),
        stats,
        type: 'js',
        isSource: true,
      }),
      options: DEFAULT_OPTIONS,
    });

    asset.addDependency({specifier: './foo', specifierType: 'esm'});
    asset.addDependency({
      specifier: './foo',
      specifierType: 'esm',
      env: {context: 'web-worker', engines: {}},
    });
    let dependencies = asset.getDependencies();
    assert(dependencies.length === 2);
  });
});
