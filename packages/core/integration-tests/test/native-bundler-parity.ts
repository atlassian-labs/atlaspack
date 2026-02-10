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

  it('creates a shared bundle for a common dependency', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-smoke-shared
        index.js:
          import('./foo');
          import('./bar');
          export default 1;
        foo.js:
          import shared from './shared';
          export default shared + 'foo';
        bar.js:
          import shared from './shared';
          export default shared + 'bar';
        shared.js:
          export default 'shared';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 0,
              "minBundleSize": 0,
              "maxParallelRequests": 100
            }
          }
        yarn.lock: {}
    `;

    let b = await bundle(
      path.join(__dirname, 'native-bundling-smoke-shared/index.js'),
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
      {type: 'js', assets: ['foo.js']},
      {type: 'js', assets: ['bar.js']},
      {type: 'js', assets: ['shared.js']},
    ]);
    await run(b);
  });

  it('creates a shared bundle for multiple common dependencies', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-smoke-shared-multi
        index.js:
          import('./foo');
          import('./bar');
          export default 1;
        foo.js:
          import a from './a';
          import b from './b';
          export default a + b + 'foo';
        bar.js:
          import a from './a';
          import b from './b';
          export default a + b + 'bar';
        a.js:
          export default 'a';
        b.js:
          export default 'b';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 0,
              "minBundleSize": 0,
              "maxParallelRequests": 100
            }
          }
        yarn.lock: {}
    `;

    let b = await bundle(
      path.join(__dirname, 'native-bundling-smoke-shared-multi/index.js'),
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
      {type: 'js', assets: ['foo.js']},
      {type: 'js', assets: ['bar.js']},
      {type: 'js', assets: ['a.js', 'b.js']},
    ]);
    await run(b);
  });
});
