// @flow
import type {BackendType, WorkerImpl} from './types';

export function detectBackend(): BackendType {
  // $FlowFixMe
  if (process.browser) return 'web';

  switch (process.env.ATLASPACK_WORKER_BACKEND) {
    case 'threads':
    case 'process':
      return process.env.ATLASPACK_WORKER_BACKEND;
  }

  // default to threads if no explicit option has been passed
  return 'threads';
}

export function getWorkerBackend(backend: BackendType): Class<WorkerImpl> {
  switch (backend) {
    case 'threads':
      return require('./threads/ThreadsWorker').default;
    case 'process':
      return require('./process/ProcessWorker').default;
    default:
      throw new Error(`Invalid backend: ${backend}`);
  }
}
