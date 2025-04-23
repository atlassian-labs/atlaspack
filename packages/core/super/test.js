const {Atlaspack} = require('./lib/core');

(async () => {
  console.log('new Atlaspack');
  const bundler = new Atlaspack({entries: './dummy.js'});

  console.log('bundler.run');
  await bundler.run();
  console.log('bundler.run done');
})();
