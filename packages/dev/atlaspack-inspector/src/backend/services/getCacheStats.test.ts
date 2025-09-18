import assert from 'assert';
import * as sinon from 'sinon';
import {getCacheStats} from './getCacheStats';

describe('getCacheStats', function () {
  let sandbox: sinon.SinonSandbox;
  let mockCache: any;

  beforeEach(() => {
    sandbox = sinon.createSandbox();
    mockCache = {
      keys: sandbox.stub(),
      getBlobSync: sandbox.stub(),
    };
  });

  afterEach(() => {
    sandbox.restore();
  });

  it('should return empty stats for empty cache', function () {
    mockCache.keys.returns([]);

    const result = getCacheStats(mockCache);

    assert.deepEqual(result, {
      size: 0,
      count: 0,
      keySize: 0,
      assetContentCount: 0,
      assetContentSize: 0,
      assetMapCount: 0,
      assetMapSize: 0,
    });
  });

  it('should calculate stats for cache with regular keys', function () {
    const mockKeys = ['key1', 'key2', 'key3'];
    const mockValues = [
      Buffer.from('value1'),
      Buffer.from('value2'),
      Buffer.from('value3'),
    ];

    mockCache.keys.returns(mockKeys);
    mockCache.getBlobSync.onCall(0).returns(mockValues[0]);
    mockCache.getBlobSync.onCall(1).returns(mockValues[1]);
    mockCache.getBlobSync.onCall(2).returns(mockValues[2]);

    const result = getCacheStats(mockCache);

    const expectedSize =
      mockValues[0].length + mockValues[1].length + mockValues[2].length;
    const expectedKeySize =
      Buffer.from('key1').length +
      Buffer.from('key2').length +
      Buffer.from('key3').length;

    assert.equal(result.size, expectedSize);
    assert.equal(result.count, 3);
    assert.equal(result.keySize, expectedKeySize);
    assert.equal(result.assetContentCount, 0);
    assert.equal(result.assetContentSize, 0);
    assert.equal(result.assetMapCount, 0);
    assert.equal(result.assetMapSize, 0);
  });

  it('should count asset content keys correctly', function () {
    const mockKeys = ['asset1:content', 'asset2:content', 'other-key'];
    const mockValues = [
      Buffer.from('content1'),
      Buffer.from('content2'),
      Buffer.from('other'),
    ];

    mockCache.keys.returns(mockKeys);
    mockCache.getBlobSync.onCall(0).returns(mockValues[0]);
    mockCache.getBlobSync.onCall(1).returns(mockValues[1]);
    mockCache.getBlobSync.onCall(2).returns(mockValues[2]);

    const result = getCacheStats(mockCache);

    assert.equal(result.assetContentCount, 2);
    assert.equal(
      result.assetContentSize,
      mockValues[0].length + mockValues[1].length,
    );
  });

  it('should count asset map keys correctly', function () {
    const mockKeys = ['asset1:map', 'asset2:map', 'other-key'];
    const mockValues = [
      Buffer.from('map1'),
      Buffer.from('map2'),
      Buffer.from('other'),
    ];

    mockCache.keys.returns(mockKeys);
    mockCache.getBlobSync.onCall(0).returns(mockValues[0]);
    mockCache.getBlobSync.onCall(1).returns(mockValues[1]);
    mockCache.getBlobSync.onCall(2).returns(mockValues[2]);

    const result = getCacheStats(mockCache);

    assert.equal(result.assetMapCount, 2);
    assert.equal(
      result.assetMapSize,
      mockValues[0].length + mockValues[1].length,
    );
  });

  it('should handle mixed cache keys correctly', function () {
    const mockKeys = [
      'asset1:content',
      'asset2:map',
      'regular-key',
      'asset3:content',
      'asset4:map',
    ];
    const mockValues = [
      Buffer.from('content1'),
      Buffer.from('map1'),
      Buffer.from('regular'),
      Buffer.from('content2'),
      Buffer.from('map2'),
    ];

    mockCache.keys.returns(mockKeys);
    mockValues.forEach((value, index) => {
      mockCache.getBlobSync.onCall(index).returns(value);
    });

    const result = getCacheStats(mockCache);

    const totalSize = mockValues.reduce((sum, val) => sum + val.length, 0);
    const totalKeySize = mockKeys.reduce(
      (sum, key) => sum + Buffer.from(key).length,
      0,
    );

    assert.equal(result.size, totalSize);
    assert.equal(result.count, 5);
    assert.equal(result.keySize, totalKeySize);
    assert.equal(result.assetContentCount, 2);
    assert.equal(
      result.assetContentSize,
      mockValues[0].length + mockValues[3].length,
    );
    assert.equal(result.assetMapCount, 2);
    assert.equal(
      result.assetMapSize,
      mockValues[1].length + mockValues[4].length,
    );
  });

  it('should handle empty string keys', function () {
    const mockKeys = [''];
    const mockValues = [Buffer.from('')];

    mockCache.keys.returns(mockKeys);
    mockCache.getBlobSync.onCall(0).returns(mockValues[0]);

    const result = getCacheStats(mockCache);

    assert.equal(result.size, 0);
    assert.equal(result.count, 1);
    assert.equal(result.keySize, 0);
  });
});
