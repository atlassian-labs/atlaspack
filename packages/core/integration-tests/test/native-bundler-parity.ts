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

  it('duplicates shared sync dep into all entry bundles', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-entry-duplication
        a.js:
          import shared from './shared';
          export default shared + 'a';
        b.js:
          import shared from './shared';
          export default shared + 'b';
        shared.js:
          export default 'shared';
        package.json:
          {}
        yarn.lock: {}
    `;

    let b = await bundle(
      [
        path.join(__dirname, 'native-bundling-entry-duplication/a.js'),
        path.join(__dirname, 'native-bundling-entry-duplication/b.js'),
      ],
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    // shared.js must be duplicated into both entries since either can load independently.
    assertBundles(b, [
      {
        name: 'a.js',
        type: 'js',
        assets: ['a.js', 'shared.js', 'esmodule-helpers.js'],
      },
      {
        name: 'b.js',
        type: 'js',
        assets: ['b.js', 'shared.js', 'esmodule-helpers.js'],
      },
    ]);
    await run(b);
  });

  it('internalizes async bundle when root is already sync-available', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-internalize
        index.js:
          import a from './a';
          export default a;
        a.js:
          const b = import('./b');
          import bSync from './b';
          export default bSync;
        b.js:
          export default 'b';
        package.json:
          {}
        yarn.lock: {}
    `;

    let b = await bundle(
      path.join(__dirname, 'native-bundling-internalize/index.js'),
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    // b.js is both lazy-imported and sync-imported. Since it's sync-available
    // from the entry, the async bundle for b.js should be internalized.
    // No runtime loaders needed since the dynamic import resolves internally.
    assertBundles(b, [
      {
        name: 'index.js',
        type: 'js',
        assets: ['index.js', 'a.js', 'b.js', 'esmodule-helpers.js'],
      },
    ]);
    await run(b);
  });

  it('creates separate bundle for CSS type-change dependency', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-type-change
        index.js:
          import './styles.css';
          export default 'hello';
        styles.css:
          .root { color: red; }
        package.json:
          {}
        yarn.lock: {}
    `;

    let b = await bundle(
      path.join(__dirname, 'native-bundling-type-change/index.js'),
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    // CSS import creates a type-change boundary -- separate CSS bundle.
    assertBundles(b, [
      {
        name: 'index.js',
        type: 'js',
        assets: ['index.js', 'esmodule-helpers.js'],
      },
      {
        type: 'css',
        assets: ['styles.css'],
      },
    ]);
    await run(b);
  });

  it('suppresses shared extraction when asset is available from ancestor', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-avail-suppression
        index.js:
          import vendor from './vendor';
          import('./a');
          import('./b');
          export default vendor;
        vendor.js:
          export default 'vendor';
        a.js:
          import vendor from './vendor';
          export default vendor + 'a';
        b.js:
          import vendor from './vendor';
          export default vendor + 'b';
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
      path.join(__dirname, 'native-bundling-avail-suppression/index.js'),
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    // vendor.js is sync-imported by entry AND both async roots. Since it's
    // in the entry bundle, it's available to a and b via ancestry -- no shared bundle.
    assertBundles(b, [
      {
        name: 'index.js',
        type: 'js',
        assets: [
          'index.js',
          'vendor.js',
          'esmodule-helpers.js',
          'bundle-url.js',
          'cacheLoader.js',
          'js-loader.js',
          'bundle-manifest.js',
        ],
      },
      {type: 'js', assets: ['a.js']},
      {type: 'js', assets: ['b.js']},
    ]);
    await run(b);
  });

  it('reuses existing async bundle instead of creating shared bundle', async function () {
    await fsFixture(overlayFS, __dirname)`
      native-bundling-reuse
        index.js:
          import('./a');
          import('./b');
          import('./c');
          export default 1;
        a.js:
          import c from './c';
          export default c + 'a';
        b.js:
          import c from './c';
          export default c + 'b';
        c.js:
          export default 'c';
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
      path.join(__dirname, 'native-bundling-reuse/index.js'),
      {
        mode: 'production',
        defaultTargetOptions: {shouldScopeHoist: false},
        inputFS: overlayFS,
      },
    );

    // c.js is lazy-imported by entry (has its own bundle) AND sync-imported
    // by a and b. The existing c.js bundle should be reused -- no shared bundle.
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
      {type: 'js', assets: ['a.js']},
      {type: 'js', assets: ['b.js']},
      {type: 'js', assets: ['c.js']},
    ]);
    await run(b);
  });
});
