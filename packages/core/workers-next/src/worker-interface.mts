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

  deserialize(_target: TransferItem): any {
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
  serdeArgs: number[],
];

export type HandleFunc<R = unknown, A extends Array<TransferItem> = any[]> = (
  ...args: A
) => R;

export type WorkerInternalMessage = [id: number, methodName: 'end', args: []];

export type WorkerMasterMessage =
  | [id: number, action: 0, ...WorkerMasterMessageCallMaster]
  | [id: number, action: 1, ...WorkerMasterMessageReverseHandle];

export type WorkerMasterMessageCallMaster = [
  location: string,
  args: Array<TransferItem>,
];
export type WorkerMasterMessageReverseHandle = [
  ref: number,
  args: Array<TransferItem>,
];
