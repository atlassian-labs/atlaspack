function run(api, a, b) {
  return api.callMaster({
    location: require.resolve('./master-sum.cjs'),
    args: [a, b]
  });
}

module.exports.run = run;
