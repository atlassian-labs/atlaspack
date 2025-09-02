import {Bundle} from '@atlaspack/types';
import {fsFixture, overlayFS, bundle} from '@atlaspack/test-utils';
import assert from 'assert';
import path from 'path';

describe('targets', () => {
  it('should support building targets with custom entries with a top level entry array', async () => {
    const targetDir = path.join(__dirname, 'targets');
    await overlayFS.mkdirp(targetDir);
    await fsFixture(overlayFS, targetDir)`
        common.js:
          export const common = 'MY_ENV';

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

    const bundlesByTarget: Record<string, Bundle> = {};
    for (const bundle of bundles) {
      bundlesByTarget[bundle.target.name] = bundle;
    }

    ['one', 'two'].forEach((target) => {
      const bundle = bundlesByTarget[target];
      assert(bundle, `Bundle for target ${target} not found`);
    });
  });
});
