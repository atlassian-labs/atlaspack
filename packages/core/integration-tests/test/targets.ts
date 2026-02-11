import {PackagedBundle} from '@atlaspack/types';
import {fsFixture, overlayFS, bundle} from '@atlaspack/test-utils';
import assert from 'assert';
import path from 'path';

describe('targets', () => {
  it('should support building targets with custom environment properties', async () => {
    const targetDir = path.join(__dirname, 'integration/target-envs');
    await overlayFS.mkdirp(targetDir);

    await fsFixture(overlayFS, targetDir)`
        package.json:
          {
            "name": "targets"
          }

        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "transformers": {
              "*.js": [
                // Note this exists as a real file for V3 compatibility
                "./env-transformer.js",
                "..."
              ]
            }
          }

        common.js:
          export const common = 'my env is: MY_ENV';

        input.js:
          import {common} from './common.js';
          console.log(common, 'from input');

        input2.js:
          import {common} from './common.js';
          console.log(common, 'from input2');

        input3.js:
          import {common} from './common.js';
          console.log(common, 'from input3');

        yarn.lock: {}
        `;

    const b = await bundle(
      [
        path.join(targetDir, './input.js'),
        path.join(targetDir, './input2.js'),
        path.join(targetDir, './input3.js'),
      ],
      {
        inputFS: overlayFS,
        targets: {
          one: {
            source: './input.js',
            context: 'browser',
            distDir: './dist',
            env: {
              MY_ENV: 'one',
            },
          },
          two: {
            source: ['./input2.js', './input3.js'],
            context: 'browser',
            distDir: './dist',
            env: {
              MY_ENV: 'two',
            },
          },
        },
        featureFlags: {
          allowExplicitTargetEntries: true,
        },
      },
    );

    const bundles = b.getBundles();
    // The feature flag should filter entries per target, so we should get 3 bundles:
    // - target "one" only for input.js
    // - target "two" only for input2.js and input3.js
    assert(bundles.length === 3, 'Expected 3 bundles, got ' + bundles.length);

    const bundlesByTarget: Record<string, PackagedBundle> = {};
    for (const bundle of bundles) {
      bundlesByTarget[bundle.target.name] = bundle;
    }

    for (const target of ['one', 'two']) {
      const bundle = bundlesByTarget[target];
      assert(bundle, `Bundle for target ${target} not found`);
      const bundleContents = await overlayFS.readFile(bundle.filePath, 'utf8');
      // The transformer successfully replaced MY_ENV with the target value
      // We need to check that the transformed value appears in the bundle
      // Since the bundler transforms the code structure, we look for the actual value
      // console.log(`Looking for transformed value "${target}" in bundle`);

      // Check that the target value appears in the bundle (the MY_ENV was replaced)
      assert(
        bundleContents.includes('my env is: ' + target),
        `Bundle should contain "my env is: " + ${target}`,
      );
    }
  });

  it('should support building targets with custom entries with a top level entry array', async () => {
    const targetDir = path.join(__dirname, 'targets');
    await overlayFS.mkdirp(targetDir);
    await fsFixture(overlayFS, targetDir)`
        common.js:
          export const common = 'my env is: MY_ENV';

        input.js:
          import {common} from './common.js';
          console.log(common, 'from input');

        input2.js:
          import {common} from './common.js';
          console.log(common, 'from input2');

        input3.js:
          import {common} from './common.js';
          console.log(common, 'from input3');

        yarn.lock: {}
        `;

    const b = await bundle([], {
      inputFS: overlayFS,
      targets: {
        one: {
          source: path.join(targetDir, './input.js'),
          context: 'browser',
          distDir: './dist',
          engines: {
            browsers: ['last 1 Chrome version'],
          },
        },
        two: {
          source: [
            path.join(targetDir, './input2.js'),
            path.join(targetDir, './input3.js'),
          ],
          context: 'browser',
          distDir: './dist',
          engines: {
            browsers: ['last 1 Firefox version'],
          },
        },
      },
    });

    const bundles = b.getBundles();
    // The feature flag should filter entries per target, so we should get 3 bundles:
    // - target "one" only for input.js
    // - target "two" only for input2.js and input3.js
    assert(bundles.length === 3, 'Expected 3 bundles, got ' + bundles.length);

    const bundlesByTarget: Record<string, PackagedBundle> = {};
    for (const bundle of bundles) {
      bundlesByTarget[bundle.target.name] = bundle;
    }

    for (const target of ['one', 'two']) {
      const bundle = bundlesByTarget[target];
      assert(bundle, `Bundle for target ${target} not found`);
    }
  });
});
