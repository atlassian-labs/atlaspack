// @flow
import type {WorkerPoolV3 as IWorkerPoolV3} from '@atlaspack/types';
import {Worker} from 'worker_threads';
import path from 'path';
import type {Transferable} from '@atlaspack/rust';
import {getAvailableThreads} from '@atlaspack/rust';

const WORKER_PATH = path.join(__dirname, 'worker', 'index.js');

export type WorkerPoolV3Options = {|
  workerCount?: number,
|};

export class WorkerPoolV3 implements IWorkerPoolV3 {
  #workers: Worker[];
  #napiWorkers: Array<Promise<Transferable>>;
  #workerCount: number;

  constructor({workerCount}: WorkerPoolV3Options = {workerCount: undefined}) {
    this.#workerCount = workerCount || getAvailableThreads();
    this.#workers = [];
    this.#napiWorkers = [];

    for (let i = 0; i < this.#workerCount; i++) {
      let worker = new Worker(WORKER_PATH);
      this.#workers.push(worker);
      this.#napiWorkers.push(new Promise((res) => worker.once('message', res)));
    }
  }

  workerCount(): number {
    return this.#workerCount;
  }

  getWorkers(): Promise<Array<Transferable>> {
    return Promise.all(this.#napiWorkers);
  }

  shutdown(): void {
    for (const worker of this.#workers) {
      worker.terminate();
    }
  }
}
