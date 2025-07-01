if (process.env.ATLASPACK_MOCHA_HANG_DEBUG === 'true') {
  const whyIsNodeRunning = require('why-is-node-running').default;
  // eslint-disable-next-line no-console
  console.log(`\n\n🛠️  Mocha process PID: ${process.pid}`);
  // eslint-disable-next-line no-console
  console.log(
    `🛠️  Run 'kill -SIGHUP ${process.pid}' to get information about open handles\n\n`,
  );

  process.on('SIGHUP', () => {
    whyIsNodeRunning();
  });
}

process.on('unhandledRejection', (reason) => {
  throw reason;
});
