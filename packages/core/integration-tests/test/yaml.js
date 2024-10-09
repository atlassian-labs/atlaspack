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

describe('yaml', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('files can be required in JavaScript', async function () {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        const test = require('./test.yaml');

        module.exports = function () {
          return test.a + test.b.c;
        };

      test.yaml:
        a: 1
        b:
          c: 2
    `;

    let b = await bundle(join(__dirname, 'index.js'), {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js', 'test.yaml'],
        childBundles: [
          {
            type: 'map',
          },
        ],
      },
    ]);

    let output = await run(b);
    assert.equal(typeof output, 'function');
    assert.equal(output(), 3);
  });

  it('files are minified', async function () {
    await fsFixture(overlayFS, __dirname)`
      index.yaml:
        a: 1
        b:
          c: 2
    `;

    await bundle(join(__dirname, 'index.yaml'), {
      defaultTargetOptions: {
        shouldOptimize: true,
        shouldScopeHoist: false,
      },
      inputFS: overlayFS,
    });

    let dist = await outputFS.readFile(join(distDir, 'index.js'), 'utf8');
    assert(dist.includes('{a:1,b:{c:2}}'));
  });
});
