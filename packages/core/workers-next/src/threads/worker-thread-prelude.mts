import type {MessagePort} from 'node:worker_threads';
import {parentPort, workerData, MessageChannel} from 'node:worker_threads';
import type {
  MasterCall,
  WorkerMessage,
  WorkerInternalMessage,
  TransferItem,
} from '../worker-interface.mts';
import {PromiseSubject} from '../promise-subject.mts';
import type {SerializedHandleRef} from '../handle-ref.mts';
import type {SerializedSharableReference} from '../sharable-reference.mts';

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

  const sharedReferences = new Map<number, any>();
  const sharedReferencesByValue = new Map<any, number>();
  const api = new Api(sharedReferences, sharedReferencesByValue, onEventMaster, onErrorMaster);

  // Internal messages
  async function onInternalCallback(msg: WorkerInternalMessage) {
    switch (msg[1]) {
      // End
      case 0: {
        const [id] = msg;
        sharedReferences.clear()
        sharedReferencesByValue.clear()
        onEvent.removeAllListeners('message');
        onInternal.removeAllListeners('message');
        onInternal.postMessage([id]);
        break;
      }
      // Put shared ref
      case 1: {
        const [id, , ref, data] = msg;
        sharedReferences.set(ref, data);
        sharedReferencesByValue.set(data, ref)
        onInternal.postMessage([id]);
        break;
      }
      // Delete shared ref
      case 2: {
        const [id, , ref] = msg;
        const value = sharedReferences.get(ref)
        sharedReferences.delete(ref);
        sharedReferencesByValue.delete(value);
        onInternal.postMessage([id]);
        break;
      }
    }
  }

  // RPC for the module
  async function onMessageCallback([id, methodName, args]: WorkerMessage) {
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
    [
      null,
      txOnEvent,
      txOnInternal,
      txOnError,
      txOnEventMaster,
      txOnErrorMaster,
    ],
    [txOnEvent, txOnInternal, txOnError, txOnEventMaster, txOnErrorMaster],
  );
}

type ListenerMap = Map<number, PromiseSubject<any>>;

class Api {
  #portMessage: MessagePort;
  #counter: number;
  #listeners: ListenerMap;
  #sharedReferences: Map<number, any>;
  #sharedReferencesByValue: Map<any, number>;

  constructor(
    sharedReferences: Map<number, any>,
    sharedReferencesByValue: Map<any, number>,
    portMessage: MessagePort,
    portError: MessagePort,
  ) {
    this.#portMessage = portMessage;
    this.#sharedReferences = sharedReferences;
    this.#sharedReferencesByValue = sharedReferencesByValue;
    portMessage.on('message', this.#onmessage);
    portError.on('message', this.#onerror);
    this.#counter = 0;
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
    this.#portMessage.postMessage([id, 0, options.location, options.args]);
    return resp;
  }

  runHandle<R, A extends Array<TransferItem>>(
    handle: SerializedHandleRef,
    args: A,
  ): Promise<R> {
    const id = this.#counter++;
    const resp = new PromiseSubject<R>();
    this.#listeners.set(id, resp);
    this.#portMessage.postMessage([id, 1, handle, args]);
    return resp;
  }

  getSharedReference<R = unknown>(
    ref: SerializedSharableReference,
  ): R | undefined {
    return this.#sharedReferences.get(ref);
  }

  resolveSharedReference(value: any): number | null {
    return this.#sharedReferencesByValue.get(value) ?? null
  }
}
