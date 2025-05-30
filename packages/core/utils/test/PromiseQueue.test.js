// @flow
import assert from 'assert';
import randomInt from 'random-int';

import PromiseQueue from '../src/PromiseQueue';
import sinon from 'sinon';

describe('PromiseQueue', () => {
  it('run() should resolve when all async functions in queue have completed', async () => {
    let queue = new PromiseQueue();

    let someBooleanToBeChanged = false;
    queue.add(() =>
      Promise.resolve().then(() => {
        someBooleanToBeChanged = true;
      }),
    );
    await queue.run();
    assert(someBooleanToBeChanged);
  });

  it('run() should reject if any of the async functions in the queue failed', async () => {
    let error = new Error('some failure');
    try {
      let queue = new PromiseQueue();
      queue.add(() => Promise.reject(error));
      await queue.run();
    } catch (e) {
      assert.equal(e, error);
    }
  });

  it('run() should reject if any of the async functions in the queue throwo', async () => {
    let error = new Error('some failure');
    try {
      let queue = new PromiseQueue();
      queue.add(() => {
        throw error;
      });
      await queue.run();
    } catch (e) {
      assert.equal(e, error);
    }
  });

  it('.run() should instantly resolve when the queue is empty', async () => {
    let queue = new PromiseQueue();
    await queue.run();
    // no need to assert, test will hang or throw an error if condition fails
  });

  it('.add() result should bubble into the run results', async () => {
    let queue = new PromiseQueue();
    queue.add(() => Promise.resolve(42));
    const result = await queue.run();
    assert.deepEqual(result, [42]);
  });

  it('constructor() should allow for configuration of max concurrent running functions', async () => {
    const maxConcurrent = 5;
    const queue = new PromiseQueue({maxConcurrent});
    let running = 0;

    new Array(100).fill(0).map(() =>
      queue.add(async () => {
        running++;
        assert(queue._numRunning === running);
        assert(running <= maxConcurrent);
        await Promise.resolve(randomInt(1, 10)); //sleep(randomInt(1, 10));
        running--;
      }),
    );

    await queue.run();
  });

  it('.add() should notify subscribers', async () => {
    const queue = new PromiseQueue();

    const subscribedFn = sinon.spy();
    queue.subscribeToAdd(subscribedFn);

    const promise = queue.add(() => Promise.resolve());
    await queue.run();
    await promise;

    assert(subscribedFn.called);
  });

  it('runs functions concurrently', () => {
    const queue = new PromiseQueue();

    const fn1 = sinon
      .stub()
      .returns(new Promise((resolve) => setTimeout(resolve, 5000)));

    queue.add(fn1); // queue does not work if nothing is running, this is broken behaviour
    queue.run();

    const fn2 = sinon
      .stub()
      .returns(new Promise((resolve) => setTimeout(resolve, 5000)));
    const fn3 = sinon
      .stub()
      .returns(new Promise((resolve) => setTimeout(resolve, 5000)));

    queue.add(fn2);
    queue.add(fn3);

    assert(fn1.calledOnce);
    assert(fn2.calledOnce);
    assert(fn3.calledOnce);
  });

  it('.subscribeToAdd() should allow unsubscribing', async () => {
    const queue = new PromiseQueue();

    const subscribedFn = sinon.spy();
    const unsubscribe = queue.subscribeToAdd(subscribedFn);
    unsubscribe();

    const promise = queue.add(() => Promise.resolve());
    await queue.run();
    await promise;

    assert(!subscribedFn.called);
  });
});
