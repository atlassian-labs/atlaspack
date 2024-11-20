// @flow strict-local

import path from 'path';
import expect from 'expect';
import {
  bundle,
  expectBundles,
  fsFixture,
  overlayFS,
  run,
} from '@atlaspack/test-utils';

describe('bundler-experimental', () => {
  describe('parity tests', () => {
    const bundlers = [
      '@atlaspack/bundler-default',
      '@atlaspack/bundler-experimental',
    ];

    bundlers.forEach((bundler) => {
      describe(`${bundler}`, () => {
        it('can bundle a single file into an output file', async () => {
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        index.js:
          output(1234);

        package.json:
          {}
        yarn.lock:
          {}

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "bundler": ${JSON.stringify(bundler)}
          }
    `;

          const inputDir = path.join(__dirname, 'bundler-experimental');
          const b = await bundle(path.join(inputDir, 'index.js'), {
            inputFS: overlayFS,
          });

          expectBundles(inputDir, b, [
            {
              name: 'index.js',
              assets: ['index.js'],
            },
          ]);

          let output = null;
          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(output).toEqual(1234);
        });
      });
    });
  });
});
