/**
 * Integration test for the v3 worker log-forwarding bridge.
 *
 * The @atlaspack/logger singleton is not shared across worker_threads
 * boundaries. This test verifies that the bridge installed in worker.ts
 * (logger.onLog -> parentPort.postMessage({type: 'logEvent', event}))
 * correctly forwards log events from the worker thread so the main thread
 * can re-emit them into its own logger for reporter consumption.
 */
import assert from 'assert';
import path from 'path';
import {Worker} from 'worker_threads';

const WORKER_PATH = path.join(__dirname, 'log-bridge-worker.js');

function spawnWorker(): Promise<{
  worker: Worker;
  waitForMessage: (predicate: (msg: unknown) => boolean) => Promise<unknown>;
}> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(WORKER_PATH);

    worker.once('error', reject);

    const messageQueue: unknown[] = [];
    const waiters: Array<{
      predicate: (msg: unknown) => boolean;
      resolve: (value: unknown) => void;
    }> = [];

    worker.on('message', (msg: unknown) => {
      const waiter = waiters.findIndex((w) => w.predicate(msg));
      if (waiter !== -1) {
        waiters.splice(waiter, 1)[0].resolve(msg);
      } else {
        messageQueue.push(msg);
      }
    });

    function waitForMessage(
      predicate: (msg: unknown) => boolean,
    ): Promise<unknown> {
      const queued = messageQueue.findIndex(predicate);
      if (queued !== -1) {
        return Promise.resolve(messageQueue.splice(queued, 1)[0]);
      }
      return new Promise((res) => waiters.push({predicate, resolve: res}));
    }

    // Wait for 'ready' before handing back the worker
    waitForMessage((m: any) => m?.type === 'ready').then(() =>
      resolve({worker, waitForMessage}),
    );
  });
}

function isLogEvent(msg: unknown): msg is {type: string; event: any} {
  return (
    msg != null && typeof msg === 'object' && (msg as any).type === 'logEvent'
  );
}

describe('v3 worker log-forwarding bridge', function () {
  this.timeout(10000);

  let worker: Worker;
  let waitForMessage: (
    predicate: (msg: unknown) => boolean,
  ) => Promise<unknown>;

  beforeEach(async () => {
    ({worker, waitForMessage} = await spawnWorker());
  });

  afterEach(() => {
    worker.terminate();
  });

  for (const level of ['verbose', 'info', 'warn'] as const) {
    it(`forwards a ${level} log event from the worker thread`, async () => {
      const diagnostic = {message: `${level} message`, origin: 'test-plugin'};

      worker.postMessage({type: 'emitLog', level, diagnostic});

      const msg = await waitForMessage(isLogEvent);
      assert.ok(isLogEvent(msg));
      assert.equal(msg.event.type, 'log');
      assert.equal(msg.event.level, level === 'info' ? 'info' : level);
      assert.deepEqual(msg.event.diagnostics, [diagnostic]);
    });
  }

  it('forwards an error log event from the worker thread', async () => {
    const diagnostic = {message: 'error message', origin: 'test-plugin'};

    worker.postMessage({type: 'emitLog', level: 'error', diagnostic});

    const msg = await waitForMessage(isLogEvent);
    assert.ok(isLogEvent(msg));
    assert.equal(msg.event.type, 'log');
    assert.equal(msg.event.level, 'error');
    assert.deepEqual(msg.event.diagnostics, [diagnostic]);
  });

  it('forwards multiple log events in order', async () => {
    const levels = ['verbose', 'info', 'warn', 'error'] as const;
    for (const level of levels) {
      worker.postMessage({
        type: 'emitLog',
        level,
        diagnostic: {message: `msg-${level}`, origin: 'test-plugin'},
      });
    }

    const received: string[] = [];
    for (let i = 0; i < levels.length; i++) {
      const msg = await waitForMessage(isLogEvent);
      assert.ok(isLogEvent(msg));
      received.push(msg.event.level);
    }

    assert.deepEqual(received, ['verbose', 'info', 'warn', 'error']);
  });
});
