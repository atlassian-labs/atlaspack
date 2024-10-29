const {parentPort} = require('worker_threads');

const receivedMessages = [];

parentPort.on('message', (message) => {
  if (message.type === 'registerWorker') {
    parentPort.postMessage({
      type: 'workerRegistered',
    });
  } else if (message.type === 'probeStatus') {
    parentPort.postMessage({
      type: 'status',
      status: 'test-status-ok',
      receivedMessages,
    });
  }

  receivedMessages.push(message);
  parentPort.postMessage({
    type: 'received',
    message,
  });
});
