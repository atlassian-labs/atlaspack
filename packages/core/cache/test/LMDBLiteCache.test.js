// @flow

import * as fs from 'fs';
import * as path from 'path';
import {tmpdir} from 'os';
import {LMDBLiteCache} from '../src/index';
import {deserialize, serialize} from 'v8';
import assert from 'assert';

const cacheDir = path.join(tmpdir(), 'lmdb-lite-cache-tests');

describe('LMDBLiteCache', () => {
  let cache;

  beforeEach(async () => {
    await fs.promises.rm(cacheDir, {recursive: true, force: true});
  });

  it('can be constructed', async () => {
    cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
  });

  it('can retrieve keys', async () => {
    cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
    await cache.setBlob('key', Buffer.from(serialize({value: 42})));
    const buffer = await cache.getBlob('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });

  it('can retrieve keys synchronously', async () => {
    cache = new LMDBLiteCache(path.join(cacheDir, 'retrieve_keys_test'));
    await cache.ensure();
    await cache.setBlob('key', Buffer.from(serialize({value: 42})));
    const buffer = cache.getBlobSync('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });

  it('can iterate over keys', async () => {
    cache = new LMDBLiteCache(path.join(cacheDir, 'keys_test'));
    await cache.ensure();
    await cache.setBlob('key1', Buffer.from(serialize({value: 42})));
    await cache.setBlob('key2', Buffer.from(serialize({value: 43})));
    const keys = cache.keys();
    assert.deepEqual(Array.from(keys), ['key1', 'key2']);
  });

  it('can compact databases', async () => {
    cache = new LMDBLiteCache(path.join(cacheDir, 'compact_test'));
    await cache.ensure();
    await cache.setBlob('key1', Buffer.from(serialize({value: 42})));
    await cache.setBlob('key2', Buffer.from(serialize({value: 43})));
    await cache.compact(path.join(cacheDir, 'compact_test_compacted'));

    cache = new LMDBLiteCache(path.join(cacheDir, 'compact_test_compacted'));
    await cache.ensure();
    const keys = cache.keys();
    assert.deepEqual(Array.from(keys), ['key1', 'key2']);
  });

  describe('getFileKey', () => {
    it('should return the correct key', () => {
      const target = path.join(cacheDir, 'test-file-keys');
      const cache = new LMDBLiteCache(target);
      const key = cache.getFileKey('key');
      assert.equal(key, path.join(target, 'files', 'key'));
    });

    it('should return the correct key for a key with a parent traversal', () => {
      const target = path.join(cacheDir, 'test-parent-keys');
      cache = new LMDBLiteCache(target);
      const key = cache.getFileKey('../../key');
      assert.equal(
        key,
        path.join(target, 'files', '$$__parent_dir$$/$$__parent_dir$$/key'),
      );
    });
  });

  it('can be closed and re-opened', async () => {
    cache = new LMDBLiteCache(path.join(cacheDir, 'close_and_reopen_test'));
    await cache.ensure();
    await cache.setBlob('key', Buffer.from(serialize({value: 42})));
    cache.getNativeRef().close();
    cache = new LMDBLiteCache(path.join(cacheDir, 'close_and_reopen_test'));
    await cache.ensure();
    const buffer = await cache.getBlob('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });
});
