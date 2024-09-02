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
  removeDistDirectory,
  run,
} from '@atlaspack/test-utils';

describe.v2('json', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('files can be required in JavaScript', async function () {
    await fsFixture(overlayFS)`
      index.js:
        const test = require('./test.json');

        module.exports = function () {
          return test.a + test.b;
        };

      test.json:
        {
          "a": 1,
          "b": 2
        }
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js', 'test.json'],
      },
    ]);

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert.equal(output(), 3);
  });

  it('files are minified', async function () {
    await fsFixture(overlayFS)`
      index.json:
        {
          "test": "test"
        }
    `;

    let b = await bundle('index.json', {
      defaultTargetOptions: {
        shouldOptimize: true,
        shouldScopeHoist: false,
      },
      inputFS: overlayFS,
    });

    let json = await outputFS.readFile(join(distDir, 'index.js'), 'utf8');
    assert(json.includes('{"test":"test"}'));

    let output = await run(b);
    assert.deepEqual(output, {test: 'test'});
  });
});
