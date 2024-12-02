// @flow strict-local

import assert from 'assert';
import nullthrows from 'nullthrows';
import path from 'path';
import {
  bundle,
  describe,
  fsFixture,
  it,
  run,
  overlayFS,
  removeDistDirectory,
  runBundles,
  distDir,
} from '@atlaspack/test-utils';
import sinon from 'sinon';

describe('conditional bundling', function () {
  beforeEach(async () => {
    await removeDistDirectory();
  });

  it(`when disabled, should treat importCond as a sync import`, async function () {
    const dir = path.join(__dirname, 'disabled-import-cond');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
        index.js:
          globalThis.__MCOND = (key) => ({ 'cond': true })[key];

          const result = importCond('cond', './a.js', './b.js');

          export default result;

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';
      `;

    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      featureFlags: {conditionalBundlingApi: false},
    });

    let output = await run(b);
    assert.deepEqual(output?.default, 'module-a');
  });

  it(`when disabled, should transform types in importCond`, async function () {
    const dir = path.join(__dirname, 'disabled-import-cond-types');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
        index.ts:
          globalThis.__MCOND = (key) => ({ 'cond': true })[key];

          const result = importCond<typeof import('./a.js'), typeof import('./b.js')>('cond', './a.js', './b.js');

          export default result;

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';
      `;

    let b = await bundle(path.join(dir, '/index.ts'), {
      inputFS: overlayFS,
      featureFlags: {conditionalBundlingApi: false},
    });

    let output = await run(b);
    assert.deepEqual(output?.default, 'module-a');
  });

  it.v2(
    `should have true and false deps as bundles in conditional manifest`,
    async function () {
      const dir = path.join(__dirname, 'import-cond-cond-manifest');
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "reporters": [
              "@atlaspack/reporter-conditional-manifest",
              "..."
            ]
          }

        index.js:
          const imported = importCond('cond', './a', './b');

          export const result = imported.default;

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';
      `;

      let bundleGraph = await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        featureFlags: {conditionalBundlingApi: true},
        defaultConfig: path.join(dir, '.parcelrc'),
      });

      // Load the generated manifest
      let conditionalManifest = JSON.parse(
        overlayFS
          .readFileSync(path.join(distDir, 'conditional-manifest.json'))
          .toString(),
      );

      // Get the corresponding bundle paths
      let ifTrueBundlePath =
        conditionalManifest?.['index.js']?.cond?.ifTrueBundles?.[0];
      let ifFalseBundlePath =
        conditionalManifest?.['index.js']?.cond?.ifFalseBundles?.[0];
      assert.ok(ifTrueBundlePath, 'ifTrue bundle path not set in manifest');
      assert.ok(ifFalseBundlePath, 'ifFalse bundle path not set in manifest');

      let ifTrueBundle = bundleGraph
        .getBundles()
        .find((b) => b.filePath === path.join(distDir, ifTrueBundlePath));
      let ifFalseBundle = bundleGraph
        .getBundles()
        .find((b) => b.filePath === path.join(distDir, ifFalseBundlePath));
      assert.ok(ifTrueBundle, 'ifTrue bundle not found');
      assert.ok(ifFalseBundle, 'ifFalse bundle not found');
    },
  );

  it.v2(`should use true bundle when condition is true`, async function () {
    const dir = path.join(__dirname, 'import-cond-true');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "reporters": [
              "@atlaspack/reporter-conditional-manifest",
              "..."
            ]
          }

        index.js:
          const conditions = { 'cond': true };
          globalThis.__MCOND = function(key) { return conditions[key]; }

          const imported = importCond('cond', './a', './b');

          export const result = imported;

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';
      `;

    let bundleGraph = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      featureFlags: {conditionalBundlingApi: true},
      defaultConfig: path.join(dir, '.parcelrc'),
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.js'),
      'index.js bundle not found',
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(path.join(distDir, 'conditional-manifest.json'))
        .toString(),
    );

    // Get the true bundle path
    let ifTrueBundlePath = path.join(
      distDir,
      nullthrows(
        conditionalManifest['index.js']?.cond?.ifTrueBundles?.[0],
        'ifTrue bundle not found in manifest',
      ),
    );
    let ifTrueBundle = nullthrows(
      bundleGraph.getBundles().find((b) => b.filePath === ifTrueBundlePath),
    );

    // Run the bundles and act like the webserver included the ifTrue bundles already
    let output = await runBundles(
      bundleGraph,
      entry,
      [
        [
          overlayFS.readFileSync(ifTrueBundle.filePath).toString(),
          ifTrueBundle,
        ],
        [overlayFS.readFileSync(entry.filePath).toString(), entry],
      ],
      {},
      {
        entryAsset: nullthrows(entry.getMainEntry()),
      },
    );

    assert.deepEqual(typeof output === 'object' && output?.result, 'module-a');
  });

  it.v2(`should use both conditional bundles correctly`, async function () {
    const dir = path.join(__dirname, 'import-cond-both');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "reporters": [
              "@atlaspack/reporter-conditional-manifest",
              "..."
            ]
          }
        index.js:
          const conditions = { 'cond1': true, 'cond2': false };
          globalThis.__MCOND = function(key) { return conditions[key]; }

          const imported1 = importCond('cond1', './a', './b');
          const imported2 = importCond('cond2', './c', './d');

          globalThis.result = [imported1, imported2];

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';

        c.js:
          export default 'module-c';

        d.js:
          export default 'module-d';
      `;

    let bundleGraph = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      featureFlags: {conditionalBundlingApi: true},
      defaultConfig: path.join(dir, '.parcelrc'),
      defaultTargetOptions: {
        outputFormat: 'esmodule',
        shouldScopeHoist: true,
      },
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.js'),
      'index.js bundle not found',
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(path.join(distDir, 'conditional-manifest.json'))
        .toString(),
    );

    // Get the true bundle path
    let ifTrueBundlePath = path.join(
      distDir,
      nullthrows(
        conditionalManifest['index.js']?.cond1?.ifTrueBundles?.[0],
        'ifTrue bundle not found in manifest',
      ),
    );
    let ifTrueBundle = nullthrows(
      bundleGraph.getBundles().find((b) => b.filePath === ifTrueBundlePath),
    );

    // Get the true bundle path
    let ifFalseBundlePath = path.join(
      distDir,
      nullthrows(
        conditionalManifest['index.js']?.cond2?.ifFalseBundles?.[0],
        'ifFalse bundle not found in manifest',
      ),
    );
    let ifFalseBundle = nullthrows(
      bundleGraph.getBundles().find((b) => b.filePath === ifFalseBundlePath),
    );

    // Run the bundles and act like the webserver included the ifTrue bundles already
    let output = await runBundles(
      bundleGraph,
      entry,
      [
        [
          overlayFS.readFileSync(ifTrueBundle.filePath).toString(),
          ifTrueBundle,
        ],
        [
          overlayFS.readFileSync(ifFalseBundle.filePath).toString(),
          ifFalseBundle,
        ],
        [overlayFS.readFileSync(entry.filePath).toString(), entry],
      ],
      {},
      {
        require: false,
        entryAsset: nullthrows(entry.getMainEntry()),
      },
    );

    assert.deepEqual(typeof output === 'object' && output?.result, [
      'module-a',
      'module-d',
    ]);
  });

  it.v2(
    `should load false bundle when importing dynamic bundles`,
    async function () {
      const dir = path.join(__dirname, 'import-cond-false-dynamic');
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": [
            "@atlaspack/reporter-conditional-manifest",
            "..."
          ]
        }
      index.js:
        const conditions = { 'cond': false };
        globalThis.__MCOND = function(key) { return conditions[key]; }

        globalThis.lazyImport = import('./lazy');

      lazy.js:
        const imported = importCond('cond', './a', './b');

        export default imported;

      a.js:
        export default 'module-a';

      b.js:
        export default 'module-b';
    `;

      let bundleGraph = await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        outputFS: overlayFS,
        featureFlags: {conditionalBundlingApi: true},
        defaultConfig: path.join(dir, '.parcelrc'),
        defaultTargetOptions: {
          shouldScopeHoist: true,
        },
      });

      let entry = nullthrows(
        bundleGraph.getBundles().find((b) => b.name === 'index.js'),
        'index.js bundle not found',
      );

      let output = await runBundles(
        bundleGraph,
        entry,
        [[overlayFS.readFileSync(entry.filePath).toString(), entry]],
        undefined,
        {
          require: false,
          entryAsset: nullthrows(entry.getMainEntry()),
        },
      );

      let lazyImported = await nullthrows(
        typeof output === 'object' ? output?.lazyImport : null,
        'Lazy import was not found on globalThis',
      );

      assert.deepEqual(
        typeof lazyImported === 'object' && lazyImported?.default,
        'module-b',
      );
    },
  );

  it.v2(`should load dev warning when bundle isn't loaded`, async function () {
    const dir = path.join(__dirname, 'import-cond-dev-warning');

    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
        index.js:
          const conditions = { 'cond': true };
          globalThis.__MCOND = function(key) { return conditions[key]; }

          const imported = importCond('cond', './a', './b');

          export const result = imported;

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';
      `;

    let bundleGraph = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      featureFlags: {conditionalBundlingApi: true},
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.js'),
      'index.js bundle not found',
    );

    let consoleStub = sinon.stub(console, 'error');
    try {
      // Run the bundles and don't include the prerequisite bundle

      // $FlowFixMe[prop-missing] rejects does exist
      await assert.rejects(() =>
        runBundles(
          bundleGraph,
          entry,
          [[overlayFS.readFileSync(entry.filePath).toString(), entry]],
          {},
          {
            entryAsset: nullthrows(entry.getMainEntry()),
          },
        ),
      );

      sinon.assert.calledWith(
        consoleStub,
        sinon.match('Conditional dependency was missing'),
      );
    } finally {
      consoleStub.restore();
    }
  });

  it.v2(
    `should handle loading conditional bundles when imported in different bundles`,
    async function () {
      const dir = path.join(__dirname, 'import-cond-different-bundles');
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
        .parcelrc:
          {
            "extends": "@atlaspack/config-default",
            "reporters": [
              "@atlaspack/reporter-conditional-manifest",
              "..."
            ]
          }
        index.js:
          const conditions = { 'cond1': true, 'cond2': true };
          globalThis.__MCOND = function(key) { return conditions[key]; }

          // Duplicate imports
          const imported1 = importCond('cond1', './a', './b');
          const imported2 = importCond('cond1', './a', './b');

          // Another import cond
          const imported3 = importCond('cond2', './a', './b');

          export const result = imported1;

        lazy.js:
          // Same import used in two different bundles
          const result = importCond('cond', './a', './b');

        a.js:
          export default 'module-a';

        b.js:
          export default 'module-b';
      `;

      let bundleGraph = await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        featureFlags: {conditionalBundlingApi: true},
        defaultConfig: path.join(dir, '.parcelrc'),
      });

      let entry = nullthrows(
        bundleGraph.getBundles().find((b) => b.name === 'index.js'),
        'index.js bundle not found',
      );

      // Load the generated manifest
      let conditionalManifest = JSON.parse(
        overlayFS
          .readFileSync(path.join(distDir, 'conditional-manifest.json'))
          .toString(),
      );

      // Get the true bundle path
      let ifTrueBundlePath = path.join(
        distDir,
        nullthrows(
          conditionalManifest['index.js']?.cond1?.ifTrueBundles?.[0],
          'ifTrue bundle not found in manifest',
        ),
      );
      let ifTrueBundle = nullthrows(
        bundleGraph.getBundles().find((b) => b.filePath === ifTrueBundlePath),
      );

      // Run the bundles and act like the webserver included the ifTrue bundles already
      let output = await runBundles(
        bundleGraph,
        entry,
        [
          [overlayFS.readFileSync(ifTrueBundlePath).toString(), ifTrueBundle],
          [overlayFS.readFileSync(entry.filePath).toString(), entry],
        ],
        {},
        {
          entryAsset: nullthrows(entry.getMainEntry()),
        },
      );

      assert.deepEqual(
        typeof output === 'object' && output?.result,
        'module-a',
      );
    },
  );

  it.v2(
    `should load bundles in parallel when config enabled`,
    async function () {
      const dir = path.join(__dirname, 'import-cond-parallel-enabled');
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": [
            "@atlaspack/reporter-conditional-manifest",
            "..."
          ]
        }
      package.json:
        {
          "@atlaspack/bundler-default": {
            "loadConditionalBundlesInParallel": true
          }
        }

      index.html:
        <!doctype html>
        <html>
        <head>
          <title>Test</title>
        </head>
        <body>
          <script type="module" src="./index.js"></script>
        </body>
        </html>

      index.js:
        const conditions = { 'cond1': true, 'cond2': true };
        globalThis.__MCOND = function(key) { return conditions[key]; }

        const imported1 = importCond('cond1', './a', './b');

        export default imported1;

      a.js:
        export default 'module-a';

      b.js:
        export default 'module-b';
    `;

      let bundleGraph = await bundle(path.join(dir, '/index.html'), {
        inputFS: overlayFS,
        featureFlags: {conditionalBundlingApi: true},
        defaultConfig: path.join(dir, '.parcelrc'),
      });

      let entry = nullthrows(
        bundleGraph.getBundles().find((b) => b.name === 'index.html'),
        'index.html bundle not found',
      );

      let entryJs = nullthrows(
        bundleGraph
          .getBundles()
          .find((b) => b.displayName === 'index.[hash].js'),
        'index.js bundle not found',
      );

      // Load the generated manifest
      let conditionalManifest = JSON.parse(
        overlayFS
          .readFileSync(path.join(distDir, 'conditional-manifest.json'))
          .toString(),
      );

      let entryContents = overlayFS.readFileSync(entry.filePath).toString();

      // Get the true bundle path
      let ifTrueBundleName = nullthrows(
        conditionalManifest[path.basename(entryJs.filePath)]?.cond1
          ?.ifTrueBundles?.[0],
        'ifTrue bundle not found in manifest',
      );
      assert.ok(
        entryContents.includes(ifTrueBundleName),
        'ifTrue script not found in HTML',
      );

      // Get the false bundle path
      let ifFalseBundleName = nullthrows(
        conditionalManifest[path.basename(entryJs.filePath)]?.cond1
          ?.ifFalseBundles?.[0],
        'ifFalse bundle not found in manifest',
      );
      assert.ok(
        entryContents.includes(ifFalseBundleName),
        'ifFalse script not found in HTML',
      );
    },
  );

  it.v2(
    `should load conditional bundles in entry html when enabled`,
    async function () {
      const dir = path.join(__dirname, 'import-cond-entry-html-enabled');
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
      .parcelrc:
        {
          "extends": "@atlaspack/config-default",
          "reporters": [
            "@atlaspack/reporter-conditional-manifest",
            "..."
          ]
        }
      package.json:
        {
          "@atlaspack/packager-html": {
            "evaluateRootConditionalBundles": true
          }
        }

      yarn.lock: {}

      index.html:
        <!doctype html>
        <html>
        <head>
          <title>Test</title>
        </head>
        <body>
          <script type="module" src="./index.js"></script>
        </body>
        </html>

      index.js:
        const conditions = { 'cond1': true, 'cond2': true };
        globalThis.__MCOND = function(key) { return conditions[key]; }

        const imported1 = importCond('cond1', './a', './b');

        export default imported1;

      a.js:
        export default 'module-a';

      b.js:
        export default 'module-b';
    `;

      let bundleGraph = await bundle(path.join(dir, '/index.html'), {
        inputFS: overlayFS,
        featureFlags: {conditionalBundlingApi: true},
        defaultConfig: path.join(dir, '.parcelrc'),
        defaultTargetOptions: {
          outputFormat: 'esmodule',
          shouldScopeHoist: true,
        },
      });

      let entry = nullthrows(
        bundleGraph.getBundles().find((b) => b.name === 'index.html'),
        'index.html bundle not found',
      );

      let entryJs = nullthrows(
        bundleGraph
          .getBundles()
          .find((b) => b.displayName === 'index.[hash].js'),
        'index.js bundle not found',
      );

      // Load the generated manifest
      let conditionalManifest = JSON.parse(
        overlayFS
          .readFileSync(path.join(distDir, 'conditional-manifest.json'))
          .toString(),
      );

      let entryContents = overlayFS.readFileSync(entry.filePath).toString();

      // Get the true bundle path
      let ifTrueBundleName = nullthrows(
        conditionalManifest[path.basename(entryJs.filePath)]?.cond1
          ?.ifTrueBundles?.[0],
        'ifTrue bundle not found in manifest',
      );
      assert.ok(
        entryContents.includes(ifTrueBundleName),
        'ifTrue script not found in HTML',
      );

      // Get the false bundle path
      let ifFalseBundleName = nullthrows(
        conditionalManifest[path.basename(entryJs.filePath)]?.cond1
          ?.ifFalseBundles?.[0],
        'ifFalse bundle not found in manifest',
      );
      assert.ok(
        entryContents.includes(ifFalseBundleName),
        'ifFalse script not found in HTML',
      );
    },
  );
});
