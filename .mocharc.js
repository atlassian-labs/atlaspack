const fs = require('fs');
const args = process.argv.slice(2);

function isFileArg(file) {
  return (
    /\.(j|t|cj|mj|ct|mt)s$/.test(file) ||
    (fs.existsSync(file) && fs.statSync(file).isDirectory())
  );
}

const spec = args.some(isFileArg)
  ? args.filter(isFileArg)
  : 'packages/*/!(integration-tests)/test/{*.{js,ts,cts,mts,cjs,mjs},**/*.{test,spec}.{js,ts,mts,cts,cjs,mjs}}';

module.exports = {
  spec,
  require: [
    '@atlaspack/babel-register',
    '@atlaspack/test-utils/src/mochaSetup.js',
  ],
  // TODO: Remove this when https://github.com/nodejs/node/pull/28788 is resolved
  exit: true,
};
