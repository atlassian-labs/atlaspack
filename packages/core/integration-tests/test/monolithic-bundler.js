// @flow
import path from 'path';
import {
  assertBundles,
  bundle,
  describe,
  it,
  overlayFS,
  fsFixture,
  run,
} from '@atlaspack/test-utils';
import assert from 'assert';

describe('monolithic bundler', function () {
  it.v2(
    'should not split any bundles when using singleFileOutput',
    async function () {
      const targets = {
        'single-file': {
          distDir: 'dist-single',
          __unstable_singleFileOutput: true,
        },
        'normally-split': {distDir: 'dist-normal'},
      };

      await fsFixture(overlayFS, __dirname)`
      single-file-output
        a.js:
          import {c} from './b';
          import './should-be-ignored.css';
          const ignore = () => import('./c');
        b.js:
          export const c = () => import('./c');
        c.js:
          export const c = 'c';
        should-be-ignored.css:
          * {
            color: papayawhip;
          }

        yarn.lock: {}
    `;

      let singleBundle = await bundle(
        path.join(__dirname, 'single-file-output/a.js'),
        {
          defaultTargetOptions: {shouldScopeHoist: false},
          inputFS: overlayFS,
          targets: {['single-file']: targets['single-file']},
        },
      );

      let splitBundle = await bundle(
        path.join(__dirname, 'single-file-output/a.js'),
        {
          defaultTargetOptions: {shouldScopeHoist: false},
          inputFS: overlayFS,
          targets: {['normally-split']: targets['normally-split']},
        },
      );

      // There should be a single bundle, including a, b, and c
      assertBundles(singleBundle, [
        {assets: ['a.js', 'b.js', 'c.js', 'esmodule-helpers.js']},
      ]);

      await run(singleBundle);

      // Without the property, the bundle should be split properly
      assertBundles(splitBundle, [
        {
          assets: [
            'a.js',
            'b.js',
            'bundle-url.js',
            'cacheLoader.js',
            'esmodule-helpers.js',
            'js-loader.js',
          ],
        },
        {assets: ['c.js']},
        {type: 'css', assets: ['should-be-ignored.css']},
      ]);

      await run(splitBundle);
    },
  );

  it.v2('should support isolated bundles', async function () {
    await fsFixture(overlayFS, __dirname)`
      isolated-bundles
        a.js:
          import iconUrl from './icon.svg';
          export const image = \`<img src="\${iconUrl}" />\`;
        icon.svg:
          <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <circle cx="50" cy="50" r="40" fill="green" />
          </svg>
        yarn.lock: {}
    `;

    let bundleResult = await bundle(
      path.join(__dirname, 'isolated-bundles/a.js'),
      {
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
        outputFS: overlayFS,
        mode: 'production',
        targets: {
          'isolated-bundle': {
            distDir: 'dist-isolated',
            __unstable_singleFileOutput: true,
          },
        },
      },
    );

    const svgBundle = bundleResult.getBundles().find((b) => b.type === 'svg');
    if (!svgBundle) {
      throw new Error('SVG bundle not found');
    }

    const svgFileName = path.basename(svgBundle.filePath);

    const result = await run(bundleResult);
    assert.equal(result.image, `<img src="http://localhost/${svgFileName}" />`);

    assertBundles(bundleResult, [
      {assets: ['a.js', 'bundle-url.js', 'esmodule-helpers.js']},
      {type: 'svg', assets: ['icon.svg']},
    ]);
  });
});
