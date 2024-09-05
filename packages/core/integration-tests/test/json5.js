import assert from 'assert';
import {join} from 'path';
import {
  assertBundles,
  bundle,
  describe,
  distDir,
  fsFixture,
  it,
  outputFS,
  overlayFS,
  run,
  removeDistDirectory,
} from '@atlaspack/test-utils';

describe('json5', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('files can be required in JavaScript', async function () {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        const test = require('./test.json5');

        module.exports = function () {
          return test.a + test.b;
        };

      test.json5:
        /*
         * comment
         */
        {
          a: 1, // comment
          b: 2,
        }
        /* end */
    `;

    let b = await bundle(join(__dirname, 'index.js'), {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js', 'test.json5'],
      },
    ]);

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert.equal(output(), 3);
  });

  it('files are minified', async function () {
    await fsFixture(overlayFS, __dirname)`
      index.json5:
        /*
         * comment
         */
        {
          "test": "test", // comment
        }
        /* end */
    `;

    let b = await bundle(join(__dirname, 'index.json5'), {
      defaultTargetOptions: {
        shouldOptimize: true,
        shouldScopeHoist: false,
      },
      inputFS: overlayFS,
    });

    let json = await outputFS.readFile(join(distDir, 'index.js'), 'utf8');
    assert(json.includes(`JSON.parse('{"test":"test"}')`));

    let output = await run(b);
    assert.deepEqual(output, {test: 'test'});
  });
});
