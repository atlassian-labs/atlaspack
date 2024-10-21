import {Flow} from 'flow-to-typescript-codemod';
import type {BackendType, WorkerImpl} from './types';

export function detectBackend(): BackendType {
  if (process.browser)
    return 'web';

  switch (process.env.ATLASPACK_WORKER_BACKEND) {
    case 'threads':
    case 'process':
      return process.env.ATLASPACK_WORKER_BACKEND;
  }

  try {
    require('worker_threads');
    return 'threads';
  } catch (err: any) {
    return 'process';
  }
}

export function getWorkerBackend(backend: BackendType): Flow.Class<WorkerImpl> {
  switch (backend) {
    case 'threads':
      return require('./threads/ThreadsWorker').default;
    case 'process':
      return require('./process/ProcessWorker').default;
    case 'web':
      return require('./web/WebWorker').default;
    default:
      throw new Error(`Invalid backend: ${backend}`);
  }
}
