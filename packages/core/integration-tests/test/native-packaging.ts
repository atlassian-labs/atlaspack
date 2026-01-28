import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  it,
  overlayFS,
  fsFixture,
  runBundle,
} from '@atlaspack/test-utils';

describe('Native packaging', function () {
  it('should package two SSR entries', async function () {
    await fsFixture(overlayFS, __dirname)`
      index.js:
        const init = async () => {
          return 'initialized';
        };

        const _default = async (input) => {
          return {
            message: 'hello from index',
            input,
          };
        };

        exports.init = init;
        exports.default = _default;

      index2.js:
        const init = async () => {
          return 'initialized';
        };

        const _default = async (input) => {
          return {
            message: 'hello from index2',
            input,
          };
        };

        exports.init = init;
        exports.default = _default;
    `;

    let b = await bundle(
      [path.join(__dirname, 'index.js'), path.join(__dirname, 'index2.js')],
      {
        inputFS: overlayFS,
        targets: {
          ssr: {
            context: 'tesseract',
            distDir: path.join(__dirname, 'dist'),
            outputFormat: 'commonjs',
          },
        },
        featureFlags: {
          nativePackager: true,
          nativePackagerSSRDev: true,
        },
      },
    );

    assert.equal(b.getBundles().length, 2, 'Should have 2 bundles');

    const bundle1 = b.getBundles().find((bundle) => bundle.name === 'index.js');
    const bundle2 = b
      .getBundles()
      .find((bundle) => bundle.name === 'index2.js');

    assert(bundle1, 'Bundle 1 (index.js) should exist');
    assert(bundle2, 'Bundle 2 (index2.js) should exist');

    const output1: any = await runBundle(b, bundle1, {});
    const output2: any = await runBundle(b, bundle2, {});

    assert.equal(
      await output1.init(),
      'initialized',
      'Bundle 1 init should work',
    );
    assert.equal(
      await output2.init(),
      'initialized',
      'Bundle 2 init should work',
    );

    const result1 = await output1.default({test: 'input1'});
    const result2 = await output2.default({test: 'input2'});

    assert.equal(
      result1.message,
      'hello from index',
      'Bundle 1 default should return correct message',
    );
    assert.equal(
      result1.input.test,
      'input1',
      'Bundle 1 should receive correct input',
    );
    assert.equal(
      result2.message,
      'hello from index2',
      'Bundle 2 default should return correct message',
    );
    assert.equal(
      result2.input.test,
      'input2',
      'Bundle 2 should receive correct input',
    );
  });
});
