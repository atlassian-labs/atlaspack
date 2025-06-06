/* eslint-disable no-inner-declarations */

require('@atlaspack/babel-register');
const {
  workerData,
  threadId,
  parentPort,
  isMainThread,
} = require('worker_threads');
const {LMDBLiteCache} = require('../src/index');

if (!isMainThread) {
  const cache = new LMDBLiteCache(workerData.cacheDir);

  async function onMessage() {
    try {
      cache.set(`worker_key/${threadId}`, {
        workerId: threadId,
      });

      const data = await cache.get('main_thread_key');

      parentPort.postMessage({
        mainThreadData: data,
        workerId: threadId,
      });

      setTimeout(() => {
        parentPort.postMessage({
          type: 'close',
          workerId: threadId,
        });
      }, Math.random() * 200);
    } catch (error) {
      parentPort.postMessage({
        error: error.message,
      });
    }
  }

  parentPort.on('message', onMessage);
}
