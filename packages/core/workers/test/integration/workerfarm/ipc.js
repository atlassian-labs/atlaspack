const WorkerFarm = require('#atlaspack/workers').default;

function run(api, a, b) {
  return api.callMaster({
    location: require.resolve('./master-sum.js'),
    args: [a, b]
  });
}

exports.run = run;
