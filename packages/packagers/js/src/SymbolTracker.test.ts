import assert from 'assert';
import {SymbolTracker} from './SymbolTracker';
import type {Asset} from '@atlaspack/types';

describe('SymbolTracker', () => {
  let tracker: SymbolTracker;
  let mockAsset1: Asset;
  let mockAsset2: Asset;
  let mockAsset3: Asset;

  beforeEach(() => {
    tracker = new SymbolTracker();
    mockAsset1 = {id: 'asset1'} as Asset;
    mockAsset2 = {id: 'asset2'} as Asset;
    mockAsset3 = {id: 'asset3'} as Asset;
  });

  describe('asset registration', () => {
    it('should register assets in correct scopes', () => {
      tracker.registerAsset(mockAsset1, false); // top-level
      tracker.registerAsset(mockAsset2, false); // top-level
      tracker.registerAsset(mockAsset3, true); // wrapped

      assert.equal(tracker.getAssetScope(mockAsset1), 'top-level');
      assert.equal(tracker.getAssetScope(mockAsset2), 'top-level');
      assert.equal(tracker.getAssetScope(mockAsset3), 'wrapped:asset3');
    });

    it('should group assets in same scope', () => {
      tracker.registerAsset(mockAsset1, false);
      tracker.registerAsset(mockAsset2, false);

      const scopeAssets = tracker.getScopeAssets(mockAsset1);
      assert(scopeAssets.has('asset1'));
      assert(scopeAssets.has('asset2'));
    });
  });

  describe('symbol declaration', () => {
    beforeEach(() => {
      tracker.registerAsset(mockAsset1, false);
      tracker.registerAsset(mockAsset2, false);
      tracker.registerAsset(mockAsset3, true);
    });

    it('should prevent duplicate symbols in same scope', () => {
      const symbol = '$abc123';

      assert.equal(tracker.declareSymbol(mockAsset1, symbol), true);
      assert.equal(tracker.declareSymbol(mockAsset2, symbol), false); // same scope
    });

    it('should allow same symbol in different scopes', () => {
      const symbol = '$abc123';

      assert.equal(tracker.declareSymbol(mockAsset1, symbol), true);
      assert.equal(tracker.declareSymbol(mockAsset3, symbol), true); // different scope
    });

    it('should track symbol existence correctly', () => {
      const symbol = '$abc123';

      tracker.declareSymbol(mockAsset1, symbol);

      assert.equal(tracker.isSymbolDeclared(mockAsset1, symbol), true);
      assert.equal(tracker.isSymbolDeclared(mockAsset2, symbol), true); // same scope
      assert.equal(tracker.isSymbolDeclared(mockAsset3, symbol), false); // different scope
    });
  });

  describe('scope information', () => {
    beforeEach(() => {
      tracker.registerAsset(mockAsset1, false);
      tracker.registerAsset(mockAsset2, false);
      tracker.registerAsset(mockAsset3, true);
    });

    it('should return correct scope symbols', () => {
      tracker.declareSymbol(mockAsset1, '$symbol1');
      tracker.declareSymbol(mockAsset1, '$symbol2');
      tracker.declareSymbol(mockAsset3, '$symbol3');

      const scope1Symbols = tracker.getScopeSymbols(mockAsset1);
      const scope3Symbols = tracker.getScopeSymbols(mockAsset3);

      assert(scope1Symbols.has('$symbol1'));
      assert(scope1Symbols.has('$symbol2'));
      assert(!scope1Symbols.has('$symbol3'));

      assert(scope3Symbols.has('$symbol3'));
      assert(!scope3Symbols.has('$symbol1'));
    });
  });

  describe('clear', () => {
    it('should reset all tracking data', () => {
      tracker.registerAsset(mockAsset1, false);
      tracker.declareSymbol(mockAsset1, '$symbol');

      tracker.clear();

      assert.equal(tracker.getAssetScope(mockAsset1), undefined);
      assert.equal(tracker.isSymbolDeclared(mockAsset1, '$symbol'), false);
    });
  });
});
