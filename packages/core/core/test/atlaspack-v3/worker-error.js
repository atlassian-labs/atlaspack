const {parentPort, workerData} = require('worker_threads');

parentPort.on('message', (message) => {
  if (message.type === 'registerWorker') {
    if (workerData.attempt === 1) {
      throw new Error('Failed to register worker');
    } else {
      parentPort.postMessage({
        type: 'workerRegistered',
      });
    }
  } else if (message.type === 'probeStatus') {
    parentPort.postMessage({
      type: 'status',
      status: 'ok',
    });
  }
});
