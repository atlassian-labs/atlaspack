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

  afterEach(() => {
    cache.getNativeRef().close();
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
    cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
    await cache.setBlob('key', Buffer.from(serialize({value: 42})));
    const buffer = cache.getBlobSync('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });

  it('can iterate over keys', async () => {
    cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
    await cache.setBlob('key1', Buffer.from(serialize({value: 42})));
    await cache.setBlob('key2', Buffer.from(serialize({value: 43})));
    const keys = cache.keys();
    assert.deepEqual(Array.from(keys), ['key1', 'key2']);
  });
});
