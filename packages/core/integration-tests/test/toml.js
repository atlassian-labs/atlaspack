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

describe.v2('toml', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it('files can be required in JavaScript', async function () {
    await fsFixture(overlayFS)`
      index.js:
        const test = require('./test.toml');

        module.exports = function () {
          return test.a + test.b.c;
        };

      test.toml:
        a = 1

        [b]
        c = 2
    `;

    let b = await bundle('index.js', {inputFS: overlayFS});

    assertBundles(b, [
      {
        name: 'index.js',
        assets: ['index.js', 'test.toml'],
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
    await fsFixture(overlayFS)`
      index.toml:
        a = 1

        [b]
        c = 2
    `;

    await bundle('index.toml', {
      defaultTargetOptions: {
        shouldOptimize: true,
        shouldScopeHoist: false,
      },
      inputFS: overlayFS,
    });

    let toml = await outputFS.readFile(join(distDir, 'index.js'), 'utf8');
    assert(toml.includes('{a:1,b:{c:2}}'));
  });
});
