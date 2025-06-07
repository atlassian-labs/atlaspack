import {EventEmitter} from 'node:events';
import type {Transferable} from 'node:worker_threads';

export type WorkerStatus = 'starting' | 'running' | 'ending' | 'ended';

export interface IWorker extends EventEmitter {
  onReady(): Promise<void>;
  status(): WorkerStatus;
  tasks(): number;
  exec(
    methodName: string,
    args: Array<any>,
    serdeArgs: number[],
  ): Promise<unknown>;
  end(): Promise<void>;
  flush(): Promise<void>;
  putSharableReference(ref: number): Promise<void>;
  deleteSharableReference(ref: number): Promise<void>;
  clearSharableReferences(): Promise<void>;
}

export type MasterCall = {
  /** @description the path to the module being imported, must be a default export */
  location: string;
  args: Array<any>;
};

export class Serializable {
  serialize(): TransferItem {
    throw new Error('Not Implemented');
  }
}

export type TransferItem =
  | Transferable
  | null
  | string
  | number
  | boolean
  | TransferItem[]
  | {[key: string]: TransferItem}
  | Serializable;

export type WorkerMessage = [
  id: number,
  methodName: string,
  args: TransferItem[],
];

export type HandleFunc<R = unknown, A extends Array<TransferItem> = any[]> = (
  ...args: A
) => R;

export type WorkerInternalMessage =
  | WorkerInternalEndMessage
  | WorkerInternalPutSharedRefMessage
  | WorkerInternalDeleteSharedRefMessage;

export type WorkerInternalEndMessage = [id: number, methodName: 0, args: []];
export type WorkerInternalPutSharedRefMessage = [
  id: number,
  methodName: 1,
  ref: number,
  value: TransferItem,
];
export type WorkerInternalDeleteSharedRefMessage = [
  id: number,
  methodName: 2,
  ref: number,
];

export type WorkerMasterMessage =
  | [id: number, action: 0, ...WorkerMasterMessageCallMaster]
  | [id: number, action: 1, ...WorkerMasterMessageReverseHandle]
  | [id: number, action: 2, ...WorkerMasterMessageSharableReference];

export type WorkerMasterMessageCallMaster = [
  location: string,
  args: Array<TransferItem>,
];

export type WorkerMasterMessageReverseHandle = [
  ref: number,
  args: Array<TransferItem>,
];

export type WorkerMasterMessageSharableReference = [ref: number];
