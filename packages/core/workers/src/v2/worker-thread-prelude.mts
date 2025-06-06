import {parentPort, workerData, MessageChannel, type MessagePort} from 'node:worker_threads';
import type {  MasterCall, WorkerMessage } from './worker-interface.mts';
import { PromiseSubject } from './promise-subject.mts';
import { HandleRef } from './handle-ref.mts';

main();
async function main() {
  if (!parentPort) {
    console.error('Not a worker');
    process.exit(1);
  }

  let module: any;
  try {
    module = await import(workerData);
  } catch (error) {
    parentPort.postMessage([error]);
  }

  const {port1: onEvent, port2: txOnEvent} = new MessageChannel();
  const {port1: onEventMaster, port2: txOnEventMaster} = new MessageChannel();
  const {port1: onErrorMaster, port2: txOnErrorMaster} = new MessageChannel();
  const {port1: onError, port2: txOnError} = new MessageChannel();
  const {port1: onInternal, port2: txOnInternal} = new MessageChannel();

  const internal = {
    end() {
      onEvent.removeAllListeners('message');
      onInternal.removeAllListeners('message');
    },
  };

  const api = new Api(onEventMaster, onErrorMaster)

  // Internal messages
  async function onInternalCallback([id, methodName, args]) {
    const result = await internal[methodName](...args);
    onInternal.postMessage([id, result]);
  }

  // RPC for the module
  async function onMessageCallback([id, methodName, args, serdeArgs]: WorkerMessage) {
    try {
      const result = await module[methodName](api, ...args);
      onEvent.postMessage([id, result]);
    } catch (error) {
      if (error instanceof Error) {
        const serde = {
          message: error.message,
          stack: error.stack,
        };
        onError.postMessage([id, serde]);
      } else {
        onError.postMessage([id, error]);
      }
    }
  }

  onEvent.on('message', onMessageCallback);
  onInternal.on('message', onInternalCallback);

  parentPort.postMessage(
    [null, txOnEvent, txOnInternal, txOnError, txOnEventMaster, txOnErrorMaster],
    [txOnEvent, txOnInternal, txOnError, txOnEventMaster, txOnErrorMaster],
  );
}

type ListenerMap = Map<number, PromiseSubject<any>>

class Api {
  #portMessage: MessagePort
  #counter: number;
  #listeners: ListenerMap;

  constructor(
    portMessage: MessagePort,
    portError: MessagePort
  ) {
    this.#portMessage = portMessage
    portMessage.on('message', this.#onmessage);
    portError.on('message', this.#onerror);
    this.#counter = 0
    this.#listeners = new Map();
  }

  #onmessage = ([id, payload]: [number, any]) => {
    this.#listeners.get(id)!.resolve(payload);
  };

  #onerror = ([id, payload]: [number, any]) => {
    this.#listeners.get(id)!.reject(payload);
  };

  callMaster(options: MasterCall) {
    const id = this.#counter++;
    const resp = new PromiseSubject<any>();
    this.#listeners.set(id, resp);
    this.#portMessage.postMessage([id, 0, options.location, options.args])
    return resp
  }

  runHandle<R, A extends Array<Transferable>>(handle: number, args: A): Promise<R> {
    const id = this.#counter++;
    const resp = new PromiseSubject<any>();
    this.#listeners.set(id, resp);
    this.#portMessage.postMessage([id, 1, handle, args])
    return resp
  }
}
