import path from 'path';
import assert from 'assert';
import {
  bundle,
  overlayFS,
  fsFixture,
  generateSyntheticApp,
  describe,
  it,
  setupV3Flags,
  assertBundles,
  run,
} from '@atlaspack/test-utils';

type BundleStructure = Array<{type: string; assets: string[]}>;

async function compareBundlers(
  fixtureName: string,
  entryFile: string,
): Promise<{jsBundles: BundleStructure; rustBundles: BundleStructure}> {
  const entryPath = path.join(__dirname, fixtureName, entryFile);
  const commonOpts = {
    mode: 'development' as const,
    defaultTargetOptions: {
      shouldScopeHoist: false,
    },
    inputFS: overlayFS,
  };

  let jsBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {nativeBundling: false},
  });

  let rustBundleGraph = await bundle(entryPath, {
    ...commonOpts,
    featureFlags: {nativeBundling: true},
  });

  function extractBundles(bg: any): BundleStructure {
    let bundles: BundleStructure = [];
    bg.traverseBundles((b: any) => {
      let assets: string[] = [];
      b.traverseAssets((a: any) => {
        // Keep comparison stable across systems by comparing basenames only.
        let name = path.basename(a.filePath);

        // Skip runtime/helper assets that can differ.
        if (/@swc[/\\]helpers/.test(a.filePath)) return;
        if (/runtime-[a-z0-9]{16}\.js/.test(a.filePath)) return;

        // Runtime loader helpers (bundle-url.js, cacheLoader.js, js-loader.js,
        // esmodule-helpers.js) are intentionally NOT skipped — they must match
        // between bundlers to ensure correct runtime asset placement parity.

        assets.push(name);
      });
      bundles.push({type: b.type, assets: assets.sort()});
    });

    // Sort bundles deterministically so deepEqual compares structure, not traversal order.
    bundles.sort((a, b) => {
      const aKey = a.type + ':' + a.assets.join(',');
      const bKey = b.type + ':' + b.assets.join(',');
      return aKey < bKey ? -1 : aKey > bKey ? 1 : 0;
    });

    return bundles;
  }

  let jsBundles = extractBundles(jsBundleGraph);
  let rustBundles = extractBundles(rustBundleGraph);

  // Compute diff between bundle sets for readable error messages.
  const toKey = (b: {assets: string[]}) => b.assets.join(',');
  const jsKeys = new Set(jsBundles.map(toKey));
  const rustKeys = new Set(rustBundles.map(toKey));
  const onlyInJs = jsBundles.filter((b) => !rustKeys.has(toKey(b)));
  const onlyInRust = rustBundles.filter((b) => !jsKeys.has(toKey(b)));

  const diffMsg =
    onlyInJs.length || onlyInRust.length
      ? `\n\nOnly in JS (${onlyInJs.length}):\n${onlyInJs.map((b) => `  [${b.assets.join(', ')}]`).join('\n')}` +
        `\n\nOnly in Rust (${onlyInRust.length}):\n${onlyInRust.map((b) => `  [${b.assets.join(', ')}]`).join('\n')}`
      : '';

  assert.equal(
    jsBundles.length,
    rustBundles.length,
    `Bundle count mismatch for ${fixtureName}: JS=${jsBundles.length}, Rust=${rustBundles.length}${diffMsg}`,
  );

  assert.deepEqual(
    rustBundles,
    jsBundles,
    `Bundle structure mismatch for ${fixtureName}.${diffMsg}`,
  );

  return {jsBundles, rustBundles};
}

describe('Native bundler parity', function () {
  describe('Basic cases', function () {
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

    it('diamond async dependency creates shared bundle', async function () {
      await fsFixture(overlayFS, __dirname)`
      native-bundler-parity-diamond-async-shared
        index.js:
          import('./a');
          import('./b');
          export default 1;
        a.js:
          import shared from './shared';
          export default shared + 'a';
        b.js:
          import shared from './shared';
          export default shared + 'b';
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
        path.join(
          __dirname,
          'native-bundler-parity-diamond-async-shared/index.js',
        ),
        {
          mode: 'production',
          defaultTargetOptions: {shouldScopeHoist: false},
          inputFS: overlayFS,
        },
      );

      // shared.js is used by both async bundles, so it should be extracted into its own shared bundle.
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
        {type: 'js', assets: ['shared.js']},
      ]);
      await run(b);
    });

    it('deep sync chain with async at leaf', async function () {
      await fsFixture(overlayFS, __dirname)`
      native-bundler-parity-deep-sync-chain-async-leaf
        index.js:
          import a from './a';
          export default a;
        a.js:
          import b from './b';
          export default b;
        b.js:
          export default import('./c');
        c.js:
          export default 123;
        package.json:
          {}
        yarn.lock: {}
    `;

      let b = await bundle(
        path.join(
          __dirname,
          'native-bundler-parity-deep-sync-chain-async-leaf/index.js',
        ),
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
            'a.js',
            'b.js',
            'esmodule-helpers.js',
            'bundle-url.js',
            'cacheLoader.js',
            'js-loader.js',
            'bundle-manifest.js',
          ],
        },
        {type: 'js', assets: ['c.js']},
      ]);
      await run(b);
    });

    it('async bundle with CSS type-change sibling', async function () {
      await fsFixture(overlayFS, __dirname)`
      native-bundler-parity-async-css-sibling
        index.js:
          export default import('./page');
        page.js:
          import './page.css';
          export default 'page';
        page.css:
          .root { color: red; }
        package.json:
          {}
        yarn.lock: {}
    `;

      let b = await bundle(
        path.join(
          __dirname,
          'native-bundler-parity-async-css-sibling/index.js',
        ),
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
            'css-loader.js',
            'bundle-manifest.js',
          ],
        },
        {type: 'js', assets: ['page.js']},
        {type: 'css', assets: ['page.css']},
      ]);
      await run(b);
    });

    it('three-way shared bundle', async function () {
      await fsFixture(overlayFS, __dirname)`
      native-bundler-parity-three-way-shared
        index.js:
          import('./a');
          import('./b');
          import('./c');
          export default 1;
        a.js:
          import shared from './shared';
          export default shared + 'a';
        b.js:
          import shared from './shared';
          export default shared + 'b';
        c.js:
          import shared from './shared';
          export default shared + 'c';
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
        path.join(__dirname, 'native-bundler-parity-three-way-shared/index.js'),
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
        {type: 'js', assets: ['a.js']},
        {type: 'js', assets: ['b.js']},
        {type: 'js', assets: ['c.js']},
        {type: 'js', assets: ['shared.js']},
      ]);
      await run(b);
    });

    it('available assets from entry not duplicated into async descendant bundles', async function () {
      await fsFixture(overlayFS, __dirname)`
      native-bundler-parity-available-from-entry-not-duplicated
        index.js:
          import shared from './shared';
          import('./async');
          export default shared;
        shared.js:
          export default 'shared';
        async.js:
          import shared from './shared';
          export const value = shared + '-async';
          export default import('./deep');
        deep.js:
          import shared from './shared';
          export default shared + '-deep';
        package.json:
          {}
        yarn.lock: {}
    `;

      let b = await bundle(
        path.join(
          __dirname,
          'native-bundler-parity-available-from-entry-not-duplicated/index.js',
        ),
        {
          mode: 'production',
          defaultTargetOptions: {shouldScopeHoist: false},
          inputFS: overlayFS,
        },
      );

      // shared.js is sync-imported by the entry, and also reachable from async.js and deep.js.
      // It should remain only in the entry bundle and be available to descendants via ancestry.
      assertBundles(b, [
        {
          name: 'index.js',
          type: 'js',
          assets: [
            'index.js',
            'shared.js',
            'esmodule-helpers.js',
            'bundle-url.js',
            'cacheLoader.js',
            'js-loader.js',
            'bundle-manifest.js',
          ],
        },
        {type: 'js', assets: ['async.js']},
        {type: 'js', assets: ['deep.js']},
      ]);
      await run(b);
    });

    it('bundle root is duplicated into entry-like bundles', async function () {
      await fsFixture(overlayFS, __dirname)`
      native-bundling-root-in-entry-like
        index.js:
          import('./shared-root');
          import('./route-a');
          import('./route-b');
          export default 1;
        shared-root.js:
          export default 'shared-root';
        route-a.js:
          import sr from './shared-root';
          export default sr + 'a';
        route-b.js:
          import sr from './shared-root';
          export default sr + 'b';
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
        path.join(__dirname, 'native-bundling-root-in-entry-like/index.js'),
        {
          mode: 'production',
          defaultTargetOptions: {shouldScopeHoist: false},
          inputFS: overlayFS,
        },
      );

      // shared-root.js is a lazy import target (bundle root) AND is sync-imported by
      // route-a and route-b. The bundler should reuse the existing shared-root bundle
      // rather than creating a separate shared bundle.
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
        {type: 'js', assets: ['shared-root.js']},
        {type: 'js', assets: ['route-a.js']},
        {type: 'js', assets: ['route-b.js']},
      ]);
      await run(b);
    });

    it('creates sibling bundle for sync svg import', async () => {
      let dir = path.join(__dirname, 'native-bundler-parity-image-sibling');
      await overlayFS.mkdirp(dir);
      await fsFixture(overlayFS, dir)`
      index.js:
        import icon from './icon.svg';
        output = icon;

      icon.svg:
        <svg xmlns="http://www.w3.org/2000/svg"><circle r="1"/></svg>

      yarn.lock:

      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "bundler": "@atlaspack/bundler-default"
        }
    `;

      let b = await bundle(path.join(dir, 'index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });

      // Both JS and SVG bundles should exist.
      // The SVG bundle is created as a type-change sibling with Isolated behavior.
      let bundles = b.getBundles();
      let svgBundles = bundles.filter((b) => b.type === 'svg');
      assert(
        svgBundles.length >= 1,
        `Expected at least 1 SVG bundle, got ${svgBundles.length}. Bundles: ${bundles.map((b) => `${b.type}:${b.name}`).join(', ')}`,
      );
    });
  });

  describe.v3('bundler parity (js vs native)', function () {
    it('Simple shared module: two async entry points that both import the same module', async function () {
      const fixtureName = 'bundler-parity-simple-shared';
      const entryFile = `${fixtureName}-index.js`;
      const fooFile = `${fixtureName}-foo.js`;
      const barFile = `${fixtureName}-bar.js`;
      const sharedFile = `${fixtureName}-shared.js`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${fooFile}');
          import('./${barFile}');
          export default 1;
        ${fooFile}:
          import shared from './${sharedFile}';
          export default shared + 'foo';
        ${barFile}:
          import shared from './${sharedFile}';
          export default shared + 'bar';
        ${sharedFile}:
          export default 'shared';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Entry-reachable asset shared by async roots', async function () {
      const fixtureName = 'bundler-parity-entry-reachable-shared-async-roots';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import shared from './shared-util.js';
          import('./page-a.js');
          import('./page-b.js');
          export default shared;

        shared-util.js:
          export default 'shared';

        page-a.js:
          import shared from './shared-util.js';
          export default shared + '-a';

        page-b.js:
          import shared from './shared-util.js';
          export default shared + '-b';

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      // Parity check: expected to FAIL today (AFB-1794): JS bundler will consider sharing
      // `shared-util.js` across async roots even though it's sync-reachable from the entry,
      // while the native bundler currently skips it due to an entry-reachable shortcut.
      await compareBundlers(fixtureName, entryFile);
    });

    it('Diamond dependency: Entry → async A, async B → both import shared C and D', async function () {
      const fixtureName = 'bundler-parity-diamond';
      const entryFile = `${fixtureName}-index.js`;
      const aFile = `${fixtureName}-a.js`;
      const bFile = `${fixtureName}-b.js`;
      const cFile = `${fixtureName}-c.js`;
      const dFile = `${fixtureName}-d.js`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          import('./${bFile}');
          export default 1;
        ${aFile}:
          import c from './${cFile}';
          import d from './${dFile}';
          export default c + d + 'a';
        ${bFile}:
          import c from './${cFile}';
          import d from './${dFile}';
          export default c + d + 'b';
        ${cFile}:
          export default 'c';
        ${dFile}:
          export default 'd';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Transitive shared: Entry → async A → sync B and Entry → async C → sync B', async function () {
      const fixtureName = 'bundler-parity-transitive-shared';
      const entryFile = `${fixtureName}-index.js`;
      const aFile = `${fixtureName}-a.js`;
      const cFile = `${fixtureName}-c.js`;
      const bFile = `${fixtureName}-b.js`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          import('./${cFile}');
          export default 1;
        ${aFile}:
          import b from './${bFile}';
          export default b + 'a';
        ${cFile}:
          import b from './${bFile}';
          export default b + 'c';
        ${bFile}:
          export default 'b';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Mixed types: JS entry imports CSS', async function () {
      const fixtureName = 'bundler-parity-mixed-types';
      const entryFile = `${fixtureName}-index.js`;
      const cssFile = `${fixtureName}-style.css`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import './${cssFile}';
          export default 1;
        ${cssFile}:
          .${fixtureName} { color: red; }
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Deep async chain: Entry → async A → async B → async C, with a module shared by A and C', async function () {
      const fixtureName = 'bundler-parity-deep-async-chain';
      const entryFile = `${fixtureName}-index.js`;
      const aFile = `${fixtureName}-a.js`;
      const bFile = `${fixtureName}-b.js`;
      const cFile = `${fixtureName}-c.js`;
      const sharedFile = `${fixtureName}-shared.js`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          export default 1;
        ${aFile}:
          import shared from './${sharedFile}';
          import('./${bFile}');
          export default shared + 'a';
        ${bFile}:
          import('./${cFile}');
          export default 'b';
        ${cFile}:
          import shared from './${sharedFile}';
          export default shared + 'c';
        ${sharedFile}:
          export default 'shared';
        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Many shared roots: 4+ async imports with overlapping shared subsets', async function () {
      const fixtureName = 'bundler-parity-many-shared-roots';
      const entryFile = `${fixtureName}-index.js`;

      const aFile = `${fixtureName}-a.js`;
      const bFile = `${fixtureName}-b.js`;
      const cFile = `${fixtureName}-c.js`;
      const dFile = `${fixtureName}-d.js`;

      const s1File = `${fixtureName}-s1.js`;
      const s2File = `${fixtureName}-s2.js`;
      const s3File = `${fixtureName}-s3.js`;
      const s4File = `${fixtureName}-s4.js`;
      const leaf1File = `${fixtureName}-leaf1.js`;
      const leaf2File = `${fixtureName}-leaf2.js`;
      const leaf3File = `${fixtureName}-leaf3.js`;
      const leaf4File = `${fixtureName}-leaf4.js`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./${aFile}');
          import('./${bFile}');
          import('./${cFile}');
          import('./${dFile}');
          export default 1;

        # Overlap matrix:
        # A: s1, s2
        # B: s2, s3
        # C: s1, s3, s4
        # D: s3, s4

        ${aFile}:
          import s1 from './${s1File}';
          import s2 from './${s2File}';
          import leaf1 from './${leaf1File}';
          export default s1 + s2 + leaf1;
        ${bFile}:
          import s2 from './${s2File}';
          import s3 from './${s3File}';
          import leaf2 from './${leaf2File}';
          export default s2 + s3 + leaf2;
        ${cFile}:
          import s1 from './${s1File}';
          import s3 from './${s3File}';
          import s4 from './${s4File}';
          import leaf3 from './${leaf3File}';
          export default s1 + s3 + s4 + leaf3;
        ${dFile}:
          import s3 from './${s3File}';
          import s4 from './${s4File}';
          import leaf4 from './${leaf4File}';
          export default s3 + s4 + leaf4;

        ${s1File}:
          export default 's1';
        ${s2File}:
          export default 's2';
        ${s3File}:
          export default 's3';
        ${s4File}:
          export default 's4';

        ${leaf1File}:
          export default 'leaf1';
        ${leaf2File}:
          export default 'leaf2';
        ${leaf3File}:
          export default 'leaf3';
        ${leaf4File}:
          export default 'leaf4';

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Complex async graph with shared bundles should not crash during naming', async function () {
      const fixtureName = 'bundler-parity-complex-async-shared';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');
          import('./page-b.js');
          import('./page-c.js');

        page-a.js:
          import { helper } from './utils/helper.js';
          import { format } from './utils/format.js';
          export default helper() + format();

        page-b.js:
          import { helper } from './utils/helper.js';
          import { validate } from './utils/validate.js';
          export default helper() + validate();

        page-c.js:
          import { format } from './utils/format.js';
          import { validate } from './utils/validate.js';
          export default format() + validate();

        utils/helper.js:
          import { common } from './common.js';
          export function helper() { return common() + 'helper'; }

        utils/format.js:
          import { common } from './common.js';
          export function format() { return common() + 'format'; }

        utils/validate.js:
          import { common } from './common.js';
          export function validate() { return common() + 'validate'; }

        utils/common.js:
          export function common() { return 'common'; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Async graph with type-change siblings should not crash during naming', async function () {
      const fixtureName = 'bundler-parity-async-type-change-siblings';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');
          import('./page-b.js');
          import('./page-c.js');

        page-a.js:
          import './styles/page-a.css';
          import './styles/shared.css';
          import { helper } from './utils/helper.js';
          import { format } from './utils/format.js';
          export default helper() + format();

        page-b.js:
          import './styles/page-b.css';
          import { helper } from './utils/helper.js';
          import { validate } from './utils/validate.js';
          export default helper() + validate();

        page-c.js:
          import { format } from './utils/format.js';
          import { validate } from './utils/validate.js';
          export default format() + validate();

        utils/helper.js:
          import { common } from './common.js';
          export function helper() { return common() + 'helper'; }

        utils/format.js:
          import { common } from './common.js';
          export function format() { return common() + 'format'; }

        utils/validate.js:
          import { common } from './common.js';
          export function validate() { return common() + 'validate'; }

        utils/common.js:
          export function common() { return 'common'; }

        styles/page-a.css:
          @import './shared.css';
          .page-a { color: red; }

        styles/page-b.css:
          @import './shared.css';
          .page-b { color: blue; }

        styles/shared.css:
          .shared { margin: 0; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('Generated app: ~150 assets with realistic structure', async function () {
      this.timeout(60000);
      const {fixtureName, entryFile} = await generateSyntheticApp(
        overlayFS,
        __dirname,
        150,
        42,
      );
      await compareBundlers(fixtureName, entryFile);
    });

    it('Generated app: ~1000 assets with realistic structure', async function () {
      this.timeout(300000);
      const {fixtureName, entryFile} = await generateSyntheticApp(
        overlayFS,
        __dirname,
        1000,
        123,
      );
      await compareBundlers(fixtureName, entryFile);
    });

    it('async import inside shared bundle should not crash during naming', async function () {
      // Repro for: bundle exists in a bundle group but is unreachable via traverseBundles,
      // so it never gets named by the Namer and crashes later in JSRuntime.getLoaderForBundle.
      const fixtureName = 'bundler-parity-async-in-shared';
      const entryFile = `${fixtureName}-index.js`;
      const asyncAFile = `${fixtureName}-async-a.js`;
      const asyncBFile = `${fixtureName}-async-b.js`;
      const sharedFile = `${fixtureName}-shared.js`;
      const localeFile = `${fixtureName}-locale.js`;

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          // Two async boundaries that both import the same shared module.
          import('./${asyncAFile}');
          import('./${asyncBFile}');

        ${sharedFile}:
          export const shared = 1;

          // Async import from within a module that should end up in a shared bundle.
          import('./${localeFile}');

        ${asyncAFile}:
          import {shared} from './${sharedFile}';

          export default shared;

        ${asyncBFile}:
          import {shared} from './${sharedFile}';

          export default shared;

        ${localeFile}:
          export const locale = 'hello';

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      await compareBundlers(fixtureName, entryFile);
    });

    it('CSS sibling merging: async import with multiple CSS dependencies', async function () {
      const fixtureName = 'bundler-parity-css-sibling-merge-async-multi';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');

        page-a.js:
          import './style-a.css';
          import './style-b.css';
          export default 'page-a';

        style-a.css:
          .a { color: red; }

        style-b.css:
          .b { color: blue; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      let {jsBundles} = await compareBundlers(fixtureName, entryFile);

      let cssBundles = jsBundles.filter((b) => b.type === 'css');
      assert.equal(cssBundles.length, 1);
      assert.deepEqual(cssBundles[0].assets, ['style-a.css', 'style-b.css']);

      // Total: JS entry + async JS + merged CSS sibling.
      assert.equal(jsBundles.length, 3);
    });

    it('SVG assets remain as separate isolated bundles', async function () {
      const fixtureName = 'bundler-parity-svg-isolated-url';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          const url1 = new URL('./icon-a.svg', import.meta.url);
          const url2 = new URL('./icon-b.svg', import.meta.url);
          export { url1, url2 };

        icon-a.svg:
          <svg><circle r="10"/></svg>

        icon-b.svg:
          <svg><rect width="10" height="10"/></svg>

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      let {jsBundles} = await compareBundlers(fixtureName, entryFile);

      let svgBundles = jsBundles.filter((b) => b.type === 'svg');
      assert.equal(svgBundles.length, 2);

      let svgAssetSets = svgBundles.map((b) => b.assets.join(','));
      assert(svgAssetSets.includes('icon-a.svg'));
      assert(svgAssetSets.includes('icon-b.svg'));

      // Total: JS entry + 2 isolated SVG bundles.
      assert.equal(jsBundles.length, 3);
    });

    it('CSS shared across async entries: same CSS imported by two async JS entries', async function () {
      this.timeout(30000);
      const fixtureName = 'bundler-parity-css-shared-across-async-entries';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');
          import('./page-b.js');

        page-a.js:
          import './shared-styles.css';
          export default 'a';

        page-b.js:
          import './shared-styles.css';
          export default 'b';

        shared-styles.css:
          .shared { color: red; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      let {jsBundles} = await compareBundlers(fixtureName, entryFile);

      let cssBundles = jsBundles.filter((b) => b.type === 'css');
      assert.equal(cssBundles.length, 1);
      assert.deepEqual(cssBundles[0].assets, ['shared-styles.css']);
    });

    it('CSS @import chains: CSS that imports other CSS from same JS entry', async function () {
      this.timeout(30000);
      const fixtureName = 'bundler-parity-css-import-chains';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page.js');

        page.js:
          import './main.css';
          export default 'page';

        main.css:
          @import './base.css';
          .main { color: blue; }

        base.css:
          .base { font-size: 16px; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      let {jsBundles} = await compareBundlers(fixtureName, entryFile);

      let cssBundles = jsBundles.filter((b) => b.type === 'css');
      assert.equal(cssBundles.length, 1);
      assert.deepEqual(cssBundles[0].assets, ['base.css', 'main.css']);

      // Total: JS entry + async JS + merged CSS.
      assert.equal(jsBundles.length, 3);
    });

    it('CSS mixed sharing: entry-specific CSS stays separate from shared CSS', async function () {
      this.timeout(30000);
      const fixtureName = 'bundler-parity-css-mixed-sharing';
      const entryFile = 'index.js';

      await fsFixture(overlayFS, __dirname)`
      ${fixtureName}
        ${entryFile}:
          import('./page-a.js');
          import('./page-b.js');

        page-a.js:
          import './page-a.css';
          import './shared.css';
          export default 'a';

        page-b.js:
          import './page-b.css';
          import './shared.css';
          export default 'b';

        page-a.css:
          .page-a { color: red; }

        page-b.css:
          .page-b { color: blue; }

        shared.css:
          .shared { color: green; }

        package.json:
          {
            "@atlaspack/bundler-default": {
              "minBundles": 1,
              "minBundleSize": 0,
              "maxParallelRequests": 99999
            }
          }
        yarn.lock:
    `;

      let {jsBundles} = await compareBundlers(fixtureName, entryFile);

      let cssBundles = jsBundles.filter((b) => b.type === 'css');
      assert.equal(cssBundles.length, 3);

      let cssAssetSets = cssBundles.map((b) => b.assets.join(','));
      assert(cssAssetSets.includes('shared.css'));
      assert(cssAssetSets.includes('page-a.css'));
      assert(cssAssetSets.includes('page-b.css'));
    });
  });
});
