// @flow strict-local

export interface WorkerPoolV3 {
  spawnWorker(): Promise<void>;
  shutdown(): Promise<void>;
}
