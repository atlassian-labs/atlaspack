const fs = require('fs');
const args = process.argv.slice(2);

function isFileArg(file) {
  return (
    /\.(j|t|cj|mj|ct|mt)s$/.test(file) ||
    (fs.existsSync(file) && fs.statSync(file).isDirectory())
  );
}

const TEST_FILE_PATTERN =
  '{*.{js,ts,cts,mts,cjs,mjs},**/*.{test,spec}.{js,ts,mts,cts,cjs,mjs}}';

const spec = args.some(isFileArg)
  ? args.filter(isFileArg)
  : [
      `packages/*/!(integration-tests|e2e-tests|atlaspack-inspector)/test/${TEST_FILE_PATTERN}`,
      `scripts/test/${TEST_FILE_PATTERN}`,
    ];

module.exports = {
  spec,
  require: [
    '@atlaspack/babel-register',
    '@atlaspack/test-utils/src/mochaSetup.js',
  ],
};
