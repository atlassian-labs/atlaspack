import url from 'node:url';

export const DEFAULT_WORKER_TIMEOUT = 3000;
export const WORKER_PATH = url.fileURLToPath(
  import.meta.resolve('#worker-threads'),
);
