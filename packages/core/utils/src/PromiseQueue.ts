import {makeDeferredWithPromise, Deferred} from './Deferred';

type PromiseQueueOpts = {
  maxConcurrent: number;
};

export default class PromiseQueue<T> {
  _deferred: Deferred<Array<T>> | null | undefined;
  _maxConcurrent: number;
  _numRunning: number = 0;
  _queue: Array<() => Promise<void>> = [];
  _runPromise: Promise<Array<T>> | null | undefined = null;
  _error: unknown;
  _count: number = 0;
  _results: Array<T> = [];
  _addSubscriptions: Set<() => void> = new Set();

  constructor(opts: PromiseQueueOpts = {maxConcurrent: Infinity}) {
    if (opts.maxConcurrent <= 0) {
      throw new TypeError('maxConcurrent must be a positive, non-zero value');
    }

    this._maxConcurrent = opts.maxConcurrent;
  }

  getNumWaiting(): number {
    return this._queue.length;
  }

  add(fn: () => Promise<T>): Promise<T> {
    return new Promise(
      (
        resolve: (result: Promise<T> | T) => void,
        reject: (error?: any) => void,
      ) => {
        let i = this._count++;
        let wrapped = () =>
          fn().then(
            (result) => {
              this._results[i] = result;
              resolve(result);
            },
            (err) => {
              reject(err);
              throw err;
            },
          );

        this._queue.push(wrapped);

        for (const addFn of this._addSubscriptions) {
          addFn();
        }

        if (this._numRunning > 0 && this._numRunning < this._maxConcurrent) {
          this._next();
        }
      },
    );
  }

  subscribeToAdd(fn: () => void): () => void {
    this._addSubscriptions.add(fn);

    return () => {
      this._addSubscriptions.delete(fn);
    };
  }

  run(): Promise<Array<T>> {
    if (this._runPromise != null) {
      return this._runPromise;
    }

    if (this._queue.length === 0) {
      return Promise.resolve([]);
    }

    let {deferred, promise} = makeDeferredWithPromise();
    this._deferred = deferred;
    // @ts-expect-error - TS2322 - Type 'Promise<unknown>' is not assignable to type 'Promise<T[]>'.
    this._runPromise = promise;

    while (this._queue.length && this._numRunning < this._maxConcurrent) {
      this._next();
    }

    // @ts-expect-error - TS2322 - Type 'Promise<unknown>' is not assignable to type 'Promise<T[]>'.
    return promise;
  }

  async _next(): Promise<void> {
    let fn = this._queue.shift();
    // @ts-expect-error - TS2345 - Argument of type '(() => Promise<void>) | undefined' is not assignable to parameter of type '() => unknown'.
    await this._runFn(fn);
    if (this._queue.length) {
      this._next();
    } else if (this._numRunning === 0) {
      this._done();
    }
  }

  async _runFn(fn: () => unknown): Promise<void> {
    this._numRunning++;
    try {
      await fn();
    } catch (e: any) {
      // Only store the first error that occurs.
      // We don't reject immediately so that any other concurrent
      // requests have time to complete.
      if (this._error == null) {
        this._error = e;
      }
    } finally {
      this._numRunning--;
    }
  }

  _resetState(): void {
    this._queue = [];
    this._count = 0;
    this._results = [];
    this._runPromise = null;
    this._numRunning = 0;
    this._deferred = null;
  }

  _done(): void {
    if (this._deferred != null) {
      if (this._error != null) {
        this._deferred.reject(this._error);
      } else {
        this._deferred.resolve(this._results);
      }
    }

    this._resetState();
  }
}
