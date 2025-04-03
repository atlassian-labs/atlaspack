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

  it.v2('should support inline bundles', async function () {
    await fsFixture(overlayFS, __dirname)`
        inline-bundles
          a.js:
            import icon from './icon.svg';
            export const image = \`<img src="\${icon}" />\`;
          icon.svg:
            <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
              <circle cx="50" cy="50" r="40" fill="green" />
            </svg>
          .parcelrc:
            {
              "extends": "@atlaspack/config-default",
              "transformers": {
                "*.svg": ["@atlaspack/transformer-inline-string"]
              },
              "optimizers": {
                "*.svg": ["@atlaspack/optimizer-data-url"]
              }
            }
          yarn.lock: {}
      `;

    let bundleResult = await bundle(
      path.join(__dirname, 'inline-bundles/a.js'),
      {
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
        mode: 'production',
        targets: {
          'inline-bundle': {
            distDir: 'dist-inline',
            __unstable_singleFileOutput: true,
          },
        },
      },
    );

    const result = await run(bundleResult);
    const expectedSvgString =
      '%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%22100%22%20height%3D%22100%22%3E%0A%20%20%3Ccircle%20cx%3D%2250%22%20cy%3D%2250%22%20r%3D%2240%22%20fill%3D%22green%22%3E%3C%2Fcircle%3E%0A%3C%2Fsvg%3E';
    assert.equal(
      result.image,
      `<img src="data:image/svg+xml,${expectedSvgString}" />`,
    );
  });

  it.v2('should support inline bundles (bundle-text)', async function () {
    await fsFixture(overlayFS, __dirname)`
        bundle-text
          a.js:
            import b from 'bundle-text:./b.js';
            export const output = \`File text: \${b}\`;
          b.js:
            export default 'Hello world';
          yarn.lock: {}
      `;

    let bundleResult = await bundle(path.join(__dirname, 'bundle-text/a.js'), {
      defaultTargetOptions: {shouldScopeHoist: false},
      inputFS: overlayFS,
      mode: 'production',
      targets: {
        'bundle-text': {
          distDir: 'dist-bundle-text',
          __unstable_singleFileOutput: true,
        },
      },
    });

    const result = await run(bundleResult);
    assert(result.output.startsWith('File text: !function(e,n,r,t,o)'));
  });
});
