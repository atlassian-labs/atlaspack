import * as path from 'node:path';
import {tmpdir} from 'os';
import {LMDBLiteCache} from '../src/index';
import {deserialize, serialize} from 'node:v8';
import assert from 'node:assert';

const cacheDir = path.join(tmpdir(), 'lmdb-lite-cache-tests');

describe('LMDBLiteCache', () => {
  it('can be constructed', async () => {
    const cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
  });

  it('can retrieve keys', async () => {
    const cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
    await cache.setBlob('key', Buffer.from(serialize({value: 42})));
    const buffer = await cache.getBlob('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });

  it('can retrieve keys synchronously', async () => {
    const cache = new LMDBLiteCache(cacheDir);
    await cache.ensure();
    cache.setBlob('key', Buffer.from(serialize({value: 42})));
    const buffer = cache.getBlobSync('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });
});
