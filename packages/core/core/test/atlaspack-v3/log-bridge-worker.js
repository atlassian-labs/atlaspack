// Minimal worker that replicates the log-forwarding bridge from worker.ts.
// Used by the log-bridge integration test only.
const {parentPort} = require('worker_threads');

if (
  process.env.ATLASPACK_SOURCES === 'true' ||
  process.env.ATLASPACK_BUILD_ENV === 'test' ||
  process.env.ATLASPACK_SELF_BUILD
) {
  require('@atlaspack/babel-register');
}

const logger = require('@atlaspack/logger').default;

// Mirror the bridge installed in worker.ts
logger.onLog((event) => {
  parentPort.postMessage({type: 'logEvent', event});
});

// Emit a log event when instructed by the test
parentPort.on('message', (message) => {
  if (message.type === 'emitLog') {
    const {level, diagnostic} = message;
    logger[level](diagnostic);
  }
});

// Signal ready
parentPort.postMessage({type: 'ready'});
