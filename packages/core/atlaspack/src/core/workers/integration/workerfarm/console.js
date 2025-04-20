// eslint-disable-next-line @atlaspack/no-self-package-imports
const WorkerFarm = require('@atlaspack/workers').default;

function run() {
  if (WorkerFarm.isWorker()) {
    // Only test this behavior in workers. Logging in the main process will
    // always work.
    console.log('one');
    console.info('two');
    console.warn('three');
    console.error('four');
    console.debug('five');
  }
}

exports.run = run;
