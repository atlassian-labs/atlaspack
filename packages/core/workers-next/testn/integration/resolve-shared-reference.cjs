function run(workerApi, ref) {
  return ref === workerApi.resolveSharedReference(workerApi.getSharedReference(ref));
}

module.exports.run = run;
