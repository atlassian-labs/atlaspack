import {EventEmitter} from 'node:events';
import type {TransferListItem} from 'node:worker_threads';

export type WorkerStatus = 'starting' | 'running' | 'ending' | 'ended';

export interface IWorker extends EventEmitter {
  onReady(): Promise<void>;
  status(): WorkerStatus;
  tasks(): number;
  exec(methodName: string, args: Array<any>, serdeArgs: number[]): Promise<unknown>;
  end(): Promise<void>;
  flush(): Promise<void>;
}

export type MasterCall = {
  /** @description the path to the module being imported, must be a default export */
  location: string;
  args: Array<any>;
};

export class Serializable {
  serialize(): Transferrable {
    throw new Error('Not Implemented');
  }

  deserialize(target: Transferrable): any {
    throw new Error('Not Implemented');
  }
}

export type Transferrable =
  | TransferListItem
  | null
  | string
  | number
  | boolean
  | Transferrable[]
  | {[key: string]: Transferrable}
  | Serializable;

export type WorkerMessage = [
  id: number,
  methodName: string,
  args: Transferrable[],
  serdeArgs: number[],
];

export type HandleFunc<R = unknown, A extends Array<Transferrable> = any[]> = (
  ...args: A
) => R;
