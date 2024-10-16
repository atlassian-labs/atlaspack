// @flow strict-local

import assert from 'assert';
import {PublicAsset, MutableAsset} from '../src/public/PublicAsset';
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

describe('Public PublicAsset', () => {
  let internalAsset;
  beforeEach(() => {
    internalAsset = new UncommittedAsset({
      options: DEFAULT_OPTIONS,
      value: createAsset({
        filePath: toProjectPath('/', '/does/not/exist'),
        code: null,
        type: 'js',
        env: createEnvironment({}),
        isSource: true,
        stats: {size: 0, time: 0},
      }),
    });
  });

  it('returns the same public PublicAsset given an internal asset', () => {
    assert.equal(
      new PublicAsset(internalAsset),
      new PublicAsset(internalAsset),
    );
  });

  it('returns the same public MutableAsset given an internal asset', () => {
    assert.equal(
      new MutableAsset(internalAsset),
      new MutableAsset(internalAsset),
    );
  });
});
