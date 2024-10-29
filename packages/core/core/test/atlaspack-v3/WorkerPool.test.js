// @flow strict-local

import path from 'path';
import {Worker} from 'worker_threads';
import {WorkerPool, waitForMessage} from '../../src/atlaspack-v3/WorkerPool';
import assert from 'assert';

function probeStatus(worker: Worker) {
  const response = waitForMessage(worker, 'status');
  worker.postMessage({type: 'probeStatus'});
  return response;
}

describe('WorkerPool', () => {
  it('can create workers and will send them the tx_worker value', async () => {
    const workerPool = new WorkerPool(path.join(__dirname, 'worker.js'));
    const workerId = workerPool.registerWorker(0);
    const worker = workerPool.getWorker(workerId);
    const status = await probeStatus(worker);

    assert.deepEqual(status, {
      type: 'status',
      status: 'test-status-ok',
      receivedMessages: [
        {
          type: 'registerWorker',
          tx_worker: 0,
        },
      ],
    });
  });

  it('when a worker is created, it is tracked', () => {
    const workerPool = new WorkerPool(path.join(__dirname, 'worker.js'));
    const w1 = workerPool.registerWorker(0);
    const w2 = workerPool.registerWorker(0);
    const w3 = workerPool.registerWorker(0);
    assert.notEqual(w1, w2);
    assert.notEqual(w2, w3);
    assert.notEqual(w1, w3);

    assert.deepEqual(workerPool.getStats(), {
      totalWorkers: 3,
      workersInUse: 3,
    });
  });

  it('workers can be released and will then be reused', async () => {
    const workerPool = new WorkerPool(path.join(__dirname, 'worker.js'));
    const worker1 = workerPool.registerWorker(0);
    workerPool.registerWorker(5);
    assert.deepEqual(workerPool.getStats(), {
      totalWorkers: 2,
      workersInUse: 2,
    });

    // Release the worker
    workerPool.releaseWorkers([worker1]);
    assert.deepEqual(workerPool.getStats(), {
      totalWorkers: 2,
      workersInUse: 1,
    });

    const worker3 = workerPool.registerWorker(33);
    assert.equal(worker1, worker3);

    const worker = workerPool.getWorker(worker3);
    const status = await probeStatus(worker);
    assert.deepEqual(status, {
      type: 'status',
      status: 'test-status-ok',
      receivedMessages: [
        {
          type: 'registerWorker',
          tx_worker: 0,
        },
        {
          type: 'registerWorker',
          tx_worker: 33,
        },
      ],
    });
  });

  describe('shutdown', () => {
    it('terminates all workers', async () => {
      const workerPool = new WorkerPool(path.join(__dirname, 'worker.js'));
      const worker1Id = workerPool.registerWorker(0);
      const worker2Id = workerPool.registerWorker(0);
      const worker1 = workerPool.getWorker(worker1Id);
      const worker2 = workerPool.getWorker(worker2Id);

      const worker1Exit = new Promise((resolve) => {
        worker1.on('exit', () => {
          resolve(null);
        });
      });
      const worker2Exit = new Promise((resolve) => {
        worker2.on('exit', () => {
          resolve(null);
        });
      });

      workerPool.shutdown();
      assert.throws(() => {
        workerPool.getWorker(worker1Id);
      });
      assert.throws(() => {
        workerPool.getWorker(worker2Id);
      });

      await worker1Exit;
      await worker2Exit;

      assert.deepEqual(workerPool.getStats(), {
        totalWorkers: 0,
        workersInUse: 0,
      });
    });
  });
});
