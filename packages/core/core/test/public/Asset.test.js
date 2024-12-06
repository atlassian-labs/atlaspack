// @flow strict-local

import assert from 'assert';
import {MutableAsset} from '../../src/public/Asset';
import UncommittedAsset from '../../src/UncommittedAsset';

describe('MutableAsset', () => {
  const setup = () => {
    // $FlowFixMe
    const asset = new UncommittedAsset({value: {}});
    const mutableAsset = new MutableAsset(asset);
    return {mutableAsset};
  };

  describe('isDirty', () => {
    it('is false when the mutable asset is created', () => {
      const {mutableAsset} = setup();
      assert.equal(mutableAsset.hasUpdatedSource, false);
    });

    it('is set to true if the code string is changed', () => {
      const {mutableAsset} = setup();

      assert.equal(mutableAsset.hasUpdatedSource, false);
      mutableAsset.setCode('new code');
      assert.equal(mutableAsset.hasUpdatedSource, true);
    });

    it('is set to true if the ast is changed', () => {
      const {mutableAsset} = setup();

      assert.equal(mutableAsset.hasUpdatedSource, false);
      // $FlowFixMe
      mutableAsset.setAST({});
      assert.equal(mutableAsset.hasUpdatedSource, true);
    });

    it('is set to true if the source buffer is changed', () => {
      const {mutableAsset} = setup();

      assert.equal(mutableAsset.hasUpdatedSource, false);
      // $FlowFixMe
      mutableAsset.setBuffer(Buffer.from('new buffer'));
      assert.equal(mutableAsset.hasUpdatedSource, true);
    });

    it('is set to true if the source stream is changed', () => {
      const {mutableAsset} = setup();

      assert.equal(mutableAsset.hasUpdatedSource, false);
      // $FlowFixMe
      mutableAsset.setStream({pipe: () => {}});
      assert.equal(mutableAsset.hasUpdatedSource, true);
    });
  });

  describe('hasUpdatedSourceMap', () => {
    it('is false when the mutable asset is created', () => {
      const {mutableAsset} = setup();
      assert.equal(mutableAsset.hasUpdatedSourceMap, false);
    });

    it('is set to true if the source map is changed', () => {
      const {mutableAsset} = setup();

      assert.equal(mutableAsset.hasUpdatedSourceMap, false);
      // $FlowFixMe
      mutableAsset.setMap({toBuffer: () => Buffer.from('new map')});
      assert.equal(mutableAsset.hasUpdatedSourceMap, true);
    });
  });
});
