// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export type Transferable = {};

export interface NapiWorkerPool {
  getWorkers(): Promise<Array<Transferable>>;
  shutdown(): void;
}
