// @flow
import type {WorkerPoolV3 as IWorkerPoolV3} from '@atlaspack/types';

export class WorkerPoolV3 implements IWorkerPoolV3 {
  async spawnWorker(): Promise<void> {}

  async shutdown(): Promise<void> {}
}
