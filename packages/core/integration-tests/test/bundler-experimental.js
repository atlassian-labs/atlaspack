// @flow strict-local

import path from 'path';
// $FlowFixMe
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

    const graphs = new Map();
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

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              name: 'index.js',
              type: 'js',
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

        it('can bundle two files together', async () => {
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        dependency.js:
          module.exports = () => 1234;
        index.js:
          const get = require('./dependency');
          output(get());

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

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              name: 'index.js',
              type: 'js',
              assets: ['dependency.js', 'index.js'],
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

        it('can bundle async splits', async () => {
          await fsFixture(overlayFS, __dirname)`
      bundler-experimental
        async.js:
          module.exports = () => 34;
        dependency.js:
          module.exports = () => 1200;
        index.js:
          const get = require('./dependency');
          output(import('./async').then((get2) => {
            return get() + get2();
          }));

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

          // $FlowFixMe
          expectBundles(inputDir, b, [
            {
              type: 'js',
              assets: ['async.js'],
            },
            {
              type: 'js',
              name: 'index.js',
              assets: ['dependency.js', 'index.js'],
            },
          ]);

          let output = null;
          graphs.set(bundler, b);

          await run(b, {
            output: (value) => {
              output = value;
            },
          });

          expect(await output).toEqual(1234);
        });
      });
    });
  });
});
