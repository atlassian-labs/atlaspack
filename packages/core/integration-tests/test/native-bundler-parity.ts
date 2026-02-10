import path from 'path';
import {
  assertBundles,
  bundle,
  describe,
  it,
  overlayFS,
  fsFixture,
  run,
  setupV3Flags,
} from '@atlaspack/test-utils';

describe('Native bundling ready', function () {
  setupV3Flags({nativeBundling: true});

  it('bundles and runs a simple entry', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-smoke-simple
        index.js:
          export default 123;
        package.json:
          {}
        yarn.lock: {}
    `;

    let b = await bundle(
      path.join(__dirname, 'native-bundling-smoke-simple/index.js'),
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    assertBundles(b, [
      {
        name: 'index.js',
        type: 'js',
        assets: ['index.js', 'esmodule-helpers.js'],
      },
    ]);
    await run(b);
  });

  it('supports dynamic import', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-smoke-dynamic
        index.js:
          export default import('./async');
        async.js:
          export default 42;
        package.json:
          {}
        yarn.lock: {}
    `;

    let b = await bundle(
      path.join(__dirname, 'native-bundling-smoke-dynamic/index.js'),
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    assertBundles(b, [
      {
        name: 'index.js',
        type: 'js',
        assets: [
          'index.js',
          'esmodule-helpers.js',
          'bundle-url.js',
          'cacheLoader.js',
          'js-loader.js',
          'bundle-manifest.js',
        ],
      },
      {
        type: 'js',
        assets: ['async.js'],
      },
    ]);
    await run(b);
  });

  it('supports multiple entries', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-smoke-multi-entry
        a.js:
          export default 1;
        b.js:
          export default 2;
        package.json:
          {}
        yarn.lock: {}
    `;

    let b = await bundle(
      [
        path.join(__dirname, 'native-bundling-smoke-multi-entry/a.js'),
        path.join(__dirname, 'native-bundling-smoke-multi-entry/b.js'),
      ],
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    assertBundles(b, [
      {
        name: 'a.js',
        type: 'js',
        assets: ['a.js', 'esmodule-helpers.js'],
      },
      {
        name: 'b.js',
        type: 'js',
        assets: ['b.js', 'esmodule-helpers.js'],
      },
    ]);
    await run(b);
  });
});
