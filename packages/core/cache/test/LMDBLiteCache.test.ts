import * as fs from 'fs';
import * as path from 'path';
import {tmpdir} from 'os';
import {LMDBLiteCache} from '../src/index';
import {deserialize, serialize} from 'v8';
import assert from 'assert';
import {Worker} from 'worker_threads';
import {initializeMonitoring} from '@atlaspack/rust';

const cacheDir = path.join(tmpdir(), 'lmdb-lite-cache-tests');

describe('LMDBLiteCache', () => {
  let cache: any;

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
    cache = new LMDBLiteCache(path.join(cacheDir, 'close_and_reopen_test'));
    await cache.ensure();
    const buffer = await cache.getBlob('key');
    const result = deserialize(buffer);
    assert.equal(result.value, 42);
  });

  it('should NOT fail when trying to open the same database twice', async () => {
    const testDir = path.join(cacheDir, 'double_open_test');
    const cache1 = new LMDBLiteCache(testDir);
    await cache1.ensure();

    assert.doesNotThrow(() => {
      new LMDBLiteCache(testDir);
    });
  });

  it('should NOT fail when trying to open after GC', async () => {
    const testDir = path.join(cacheDir, 'gc_test');

    let cache1 = new LMDBLiteCache(testDir);
    await cache1.ensure();
    await cache1.setBlob('key', Buffer.from(serialize({value: 42})));

    cache1 = null;

    if (global.gc) {
      global.gc();
    }

    assert.doesNotThrow(() => {
      new LMDBLiteCache(testDir);
    });
  });

  it('should handle rapid open/close cycles', async () => {
    const testDir = path.join(cacheDir, 'rapid_cycles_test');

    for (let i = 0; i < 10; i++) {
      const cache = new LMDBLiteCache(testDir);
      await cache.ensure();
      await cache.setBlob(`key${i}`, Buffer.from(serialize({value: i})));

      await new Promise((resolve: any) => setTimeout(resolve, 10));
    }

    const finalCache = new LMDBLiteCache(testDir);
    await finalCache.ensure();
    const buffer = await finalCache.getBlob('key9');
    const result = deserialize(buffer);
    assert.equal(result.value, 9);
  });

  it('should work when there are multiple node.js worker threads accessing the same database', async function () {
    this.timeout(40000);

    try {
      initializeMonitoring();
    } catch {
      /* empty */
    }

    const testDir = path.join(cacheDir, 'worker_threads_test');

    let cache = new LMDBLiteCache(testDir);
    await cache.set('main_thread_key', {
      mainThreadId: 0,
      hello: 'world',
    });
    setTimeout(() => {
      cache = null;

      if (global.gc) {
        global.gc();
      }
    }, Math.random() * 300);

    const numWorkers = 10;

    const workers: Array<any> = [];
    const responsePromises: Array<any> = [];
    for (let i = 0; i < numWorkers; i++) {
      const worker = new Worker(path.join(__dirname, 'workerThreadsTest.js'), {
        workerData: {
          cacheDir: testDir,
        },
      });
      workers.push(worker);

      const responsePromise = new Promise((resolve: any, reject: any) => {
        worker.addListener('error', (error: Error) => {
          reject(error);
        });
        worker.addListener('message', (message: any) => {
          resolve(message);
        });
      });

      worker.addListener('message', (message: any) => {
        // eslint-disable-next-line no-console
        console.log('Worker message', message);
      });
      worker.addListener('online', () => {
        worker.postMessage({
          type: 'go',
        });
      });

      responsePromises.push(responsePromise);
    }

    // eslint-disable-next-line no-console
    console.log('Waiting for responses');
    const responses = await Promise.all(responsePromises);

    // eslint-disable-next-line no-console
    console.log('Responses received');
    for (const [index, response] of responses.entries()) {
      const worker = workers[index];

      assert.deepEqual(
        response,
        {
          mainThreadData: {
            mainThreadId: 0,
            hello: 'world',
          },
          workerId: worker.threadId,
        },
        `worker_${index} - Worker ${worker.threadId} should have received the correct data`,
      );
    }

    // eslint-disable-next-line no-console
    console.log('Getting main thread key');
    cache = new LMDBLiteCache(testDir);
    const data = await cache?.get('main_thread_key');
    assert.deepEqual(data, {
      mainThreadId: 0,
      hello: 'world',
    });

    // eslint-disable-next-line no-console
    console.log('Getting worker keys');
    for (const worker of workers) {
      const data = await cache?.get(`worker_key/${worker.threadId}`);
      assert.deepEqual(data, {
        workerId: worker.threadId,
      });

      await new Promise((resolve: any) => setTimeout(resolve, 500));
      worker.terminate();
    }
  });
});
