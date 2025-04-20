// @flow strict-local

export type Transferable = {||};

export interface NapiWorkerPool {
  getWorkers(): Promise<Array<Transferable>>;
  shutdown(): void;
}
