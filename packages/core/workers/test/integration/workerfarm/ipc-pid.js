const WorkerFarm = require('#atlaspack/workers').default;

function run(api) {
  let result = [process.pid];
  return new Promise((resolve, reject) => {
    api.callMaster({
      location: require.resolve('./master-process-id.js'),
      args: []
    })
      .then(pid => {
        result.push(pid);
        resolve(result);
      })
      .catch(reject);
  });
}

exports.run = run;
