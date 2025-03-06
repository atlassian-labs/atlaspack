// @flow strict-local

export type Transferable = {||};

export interface WorkerPoolV3 {
  getWorkers(): Promise<Array<Transferable>>;
  shutdown(): void;
}
