// /* eslint-disable no-console */
// console.clear()

// const assert = require('node:assert');
// const WorkerFarm = require('../lib/index.js').default;

// (async () => {
//   let workerfarm = new WorkerFarm({
//     warmWorkers: true,
//     useLocalWorker: true,
//     workerPath: require.resolve('./integration/workerfarm/echo.js'),
//   });

//   for (let i = 0; i < 100; i++) {
//     assert.equal(await workerfarm.run(i), i);
//   }

//   await new Promise((resolve) => workerfarm.once('warmedup', resolve));

//   assert(workerfarm.workers.size > 0, 'Should have spawned workers.');
//   assert(
//     workerfarm.warmWorkers >= workerfarm.workers.size,
//     'Should have warmed up workers.',
//   );

//   await workerfarm.end();
// })();
