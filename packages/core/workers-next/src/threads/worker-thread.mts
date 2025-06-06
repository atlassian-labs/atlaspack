import {EventEmitter} from 'node:events';
import type {Transferable} from 'node:worker_threads';
import {Worker} from 'node:worker_threads';
import {MessagePort} from 'node:worker_threads';
import {PromiseSubject} from '../promise-subject.mts';
import {WORKER_PATH} from '../constants.mts';
import type {
  IWorker,
  WorkerStatus,
  TransferItem,
  HandleFunc,
  WorkerMasterMessage,
} from '../worker-interface.mts';

type ListenerMap = Map<number, PromiseSubject<any>>;

export type WorkerThreadOptions = {
  workerTimeout: number;
  workerPath: string;
  reverseHandles: Array<HandleFunc>;
};

export class WorkerThread extends EventEmitter implements IWorker {
  #worker: PromiseSubject<Worker>;
  #onEvent: PromiseSubject<MessagePort>;
  #onEventMaster: PromiseSubject<MessagePort>;
  #onInternal: PromiseSubject<MessagePort>;
  #listeners: ListenerMap;
  #counter: number;
  #status: WorkerStatus;
  #reverseHandles: Array<HandleFunc>;

  constructor(options: WorkerThreadOptions) {
    super();
    this.#status = 'starting';
    this.#counter = 0;
    this.#listeners = new Map();
    this.#worker = new PromiseSubject();
    this.#onEvent = new PromiseSubject();
    this.#onInternal = new PromiseSubject();
    this.#onEventMaster = new PromiseSubject();
    this.#reverseHandles = options.reverseHandles;

    const worker = new Worker(WORKER_PATH, {
      workerData: options.workerPath,
    });

    worker.once(
      'message',
      ([error, onEvent, onInternal, onError, onEventMaster, onErrorMaster]: [
        any,
        MessagePort,
        MessagePort,
        MessagePort,
        MessagePort,
        MessagePort,
      ]) => {
        if (error) {
          this.#worker.reject(error);
          return;
        }
        onEvent.on('message', this.#onmessage);
        onEventMaster.on('message', this.#onmessagemaster);
        onInternal.on('message', this.#onmessage);
        onError.on('message', this.#onerror);
        this.#onEvent.resolve(onEvent);
        this.#onInternal.resolve(onInternal);
        this.#onEventMaster.resolve(onEventMaster);
        this.#worker.resolve(worker);
        this.emit('ready');
        this.#status = 'running';
      },
    );
  }

  async onReady(): Promise<void> {
    await this.#worker;
  }

  status(): WorkerStatus {
    return this.#status;
  }

  tasks(): number {
    return this.#listeners.size;
  }

  #onmessage = ([id, payload]: [number, any]) => {
    this.#listeners.get(id)!.resolve(payload);
  };

  #onmessagemaster = async (msg: WorkerMasterMessage) => {
    try {
      switch (msg[1]) {
        case 0: {
          const [id, , location, args] = msg;
          const module = await import(location);
          const result = await module.default(...args);
          (await this.#onEventMaster).postMessage([id, result]);
          break;
        }
        case 1: {
          const [id, , handle, args] = msg;
          const result = await this.#reverseHandles[handle](...args);
          (await this.#onEventMaster).postMessage([id, result]);
          break;
        }
      }
    } catch (error) {
      console.log(error);
    }
  };

  #onerror = ([id, payload]: [number, any]) => {
    this.#listeners.get(id)!.reject(payload);
  };

  async exec(
    methodName: string,
    args: Array<TransferItem>,
    serdeArgs: number[],
  ): Promise<unknown> {
    const [id, resp] = this.#addTask();
    const transferList: Transferable[] = args.filter(
      (arg) => arg instanceof ArrayBuffer || arg instanceof MessagePort,
    );
    (await this.#onEvent).postMessage(
      [id, methodName, args, serdeArgs],
      transferList,
    );
    return resp.finally(() => this.#listeners.delete(id));
  }

  async end(): Promise<void> {
    this.#status = 'ending';
    const [id, resp] = this.#addTask();
    this.#listeners = new ClosedListeners(this.#listeners, [id]);
    (await this.#onInternal).postMessage([id, 'end', []]);
    await resp;
    await (await this.#worker).terminate();
    this.#status = 'ended';
  }

  async flush(): Promise<void> {
    await Promise.all(Array.from(this.#listeners.values()));
  }

  #addTask(): [number, PromiseSubject<any>] {
    const id = this.#counter++;
    const resp = new PromiseSubject<any>();
    this.#listeners.set(id, resp);
    return [id, resp];
  }
}

/**
 * @description This class allows for reading and
 * removing keys but not adding new keys. This is
 * used to replace the Map containing listeners for
 * return values which is faster than doing a runtime
 * check every time a method is run to determine if
 * there is an error */
class ClosedListeners implements ListenerMap {
  #inner: Map<number, PromiseSubject<any>>;
  #allowed: number[];

  get size(): number {
    return this.#inner.size;
  }

  constructor(init: ListenerMap, allowed: number[]) {
    this.#inner = init;
    this.#allowed = allowed;
  }

  clear(): ReturnType<ListenerMap['clear']> {
    return this.#inner.clear();
  }

  forEach(
    ...args: Parameters<ListenerMap['forEach']>
  ): ReturnType<ListenerMap['forEach']> {
    return this.#inner.forEach(...args);
  }

  get(...args: Parameters<ListenerMap['get']>): ReturnType<ListenerMap['get']> {
    return this.#inner.get(...args);
  }

  has(...args: Parameters<ListenerMap['has']>): boolean {
    return this.#inner.has(...args);
  }

  entries(): ReturnType<ListenerMap['entries']> {
    return this.#inner.entries();
  }

  keys(): ReturnType<ListenerMap['keys']> {
    return this.#inner.keys();
  }

  values(): ReturnType<ListenerMap['values']> {
    return this.#inner.values();
  }

  [Symbol.iterator](): MapIterator<[number, PromiseSubject<any>]> {
    return this.#inner[Symbol.iterator]();
  }

  get [Symbol.toStringTag](): string {
    return this.#inner[Symbol.toStringTag];
  }

  set(key: number, value: PromiseSubject<any>): any {
    if (this.#allowed.includes(key)) {
      this.#inner.set(key, value);
      return this as any;
    }
    value.reject('Cannot run method, WorkerRef is closed');
    return this as any;
  }

  delete(key: number): boolean {
    return this.#inner.delete(key);
  }
}
