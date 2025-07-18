export type Transferable = Record<any, any>;

export interface NapiWorkerPool {
  getWorkers(): Promise<Array<Transferable>>;
  shutdown(): void;
}
