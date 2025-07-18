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

  it(`should have true and false deps as bundles in conditional manifest`, async function () {
    const workingDir = 'import-cond-cond-manifest';
    const dir = path.join(__dirname, workingDir);
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
            "@atlaspack/reporter-conditional-manifest": {
              "filename": "${path.join(
                workingDir,
                'conditional-manifest.json',
              )}"
            }
          }

        yarn.lock: {}

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
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
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
  });

  it(`should use true bundle when condition is true`, async function () {
    const workingDir = 'import-cond-true';
    const dir = path.join(__dirname, workingDir);
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
            "@atlaspack/reporter-conditional-manifest": {
              "filename": "${path.join(
                workingDir,
                'conditional-manifest.json',
              )}"
            }
          }

        yarn.lock: {}

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
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
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

  it(`should use both conditional bundles correctly`, async function () {
    const workingDir = 'import-cond-both';
    const dir = path.join(__dirname, workingDir);
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
            "@atlaspack/reporter-conditional-manifest": {
              "filename": "${path.join(
                workingDir,
                'conditional-manifest.json',
              )}"
            }
          }

        yarn.lock: {}
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
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.js'),
      'index.js bundle not found',
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
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

  it(`should load false bundle when importing dynamic bundles`, async function () {
    const workingDir = 'import-cond-false-dynamic';
    const dir = path.join(__dirname, workingDir);
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
          "@atlaspack/reporter-conditional-manifest": {
            "filename": "${path.join(workingDir, 'conditional-manifest.json')}"
          }
        }

      yarn.lock: {}
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
  });

  it(`should load dev warning when bundle isn't loaded`, async function () {
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
        sinon.match(
          'Conditional dependency was not registered when executing.',
        ),
      );
    } finally {
      consoleStub.restore();
    }
  });

  it(`should handle loading conditional bundles when imported in different bundles`, async function () {
    const workingDir = 'import-cond-different-bundles';
    const dir = path.join(__dirname, workingDir);
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
            "@atlaspack/reporter-conditional-manifest": {
              "filename": "${path.join(
                workingDir,
                'conditional-manifest.json',
              )}"
            }
          }

        yarn.lock: {}
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
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
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

    assert.deepEqual(typeof output === 'object' && output?.result, 'module-a');
  });

  it(`should load bundles in parallel when config enabled`, async function () {
    const workingDir = 'import-cond-parallel-enabled';
    const dir = path.join(__dirname, workingDir);
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

      yarn.lock: {}

      package.json:
        {
          "@atlaspack/bundler-default": {
            "loadConditionalBundlesInParallel": true
          },
          "@atlaspack/reporter-conditional-manifest": {
            "filename": "${path.join(workingDir, 'conditional-manifest.json')}"
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
      bundleGraph.getBundles().find((b) => b.displayName === 'index.[hash].js'),
      'index.js bundle not found',
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
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
  });

  it(`should load conditional bundles in entry html when enabled`, async function () {
    const workingDir = 'import-cond-entry-html-enabled';
    const dir = path.join(__dirname, workingDir);
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
          },
          "@atlaspack/reporter-conditional-manifest": {
            "filename": "${path.join(workingDir, 'conditional-manifest.json')}"
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
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.html'),
      'index.html bundle not found',
    );

    let entryJs = nullthrows(
      bundleGraph.getBundles().find((b) => b.displayName === 'index.[hash].js'),
      'index.js bundle not found',
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
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
  });

  it(`should load more conditional bundles in entry html when enabled`, async function () {
    const dir = path.join(__dirname, 'import-cond-entry-html-more-enabled');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
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
        const imported2 = importCond('cond2', './c', './d');
        export default 'module-a';

      b.js:
        export default 'module-b';

      c.js:
        export default 'module-c';

      d.js:
        export default 'module-d';
    `;

    let bundleGraph = await bundle(path.join(dir, '/index.html'), {
      inputFS: overlayFS,
      featureFlags: {
        conditionalBundlingApi: true,
      },
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.html'),
      'index.html bundle not found',
    );

    let entryContents = overlayFS.readFileSync(entry.filePath).toString();

    // There should be all four bundles loaded in the html
    assert.equal(entryContents.match(/data-conditional/g)?.length, 4);
  });

  it(`should fallback to loading conditional bundles sync if missing`, async function () {
    const dir = path.join(__dirname, 'import-cond-fallback-if-missing');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      yarn.lock: {}
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

    let bundleGraph = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      featureFlags: {
        conditionalBundlingApi: true,
        condbDevFallbackDev: true,
      },
    });

    let entry = nullthrows(
      bundleGraph.getBundles().find((b) => b.name === 'index.js'),
      'index.js bundle not found',
    );

    const mockXMLHttpRequest = function () {
      this.open = function (method, url, async) {
        this.method = method;
        this.url = new URL(url);
        this.async = async;
      };
      this.send = function () {
        const matchedBundle = bundleGraph.getBundles().find((b) => {
          return (
            b.filePath.slice(distDir.length + 1) ===
            this.url.pathname.replace(/^\//, '')
          );
        });
        if (matchedBundle) {
          // Simulate successful response
          this.status = 200;
          this.responseText = overlayFS
            .readFileSync(matchedBundle.filePath)
            .toString();
        } else {
          this.status = 404;
          this.responseText = '';
        }
      };
      this.status = 0;
      this.responseText = '';
    };

    // Patch setTimeout to be synchronous for this test
    const origSetTimeout = global.setTimeout;
    global.setTimeout = (fn) => fn();

    let output;
    try {
      output = await runBundles(
        bundleGraph,
        entry,
        [[overlayFS.readFileSync(entry.filePath).toString(), entry]],
        {
          XMLHttpRequest: mockXMLHttpRequest,
        },
        {
          entryAsset: nullthrows(entry.getMainEntry()),
        },
      );
    } finally {
      // Restore setTimeout
      global.setTimeout = origSetTimeout;
    }

    assert.deepEqual(typeof output === 'object' && output?.default, 'module-a');
  });

  it(`should have correct deps as bundles in conditional manifest when nested`, async function () {
    const workingDir = 'import-cond-cond-manifest-nested';
    const dir = path.join(__dirname, workingDir);
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
            "@atlaspack/reporter-conditional-manifest": {
              "filename": "${path.join(
                workingDir,
                'conditional-manifest.json',
              )}"
            }
          }

        yarn.lock: {}

        index.js:
          const imported = importCond('cond', './a', './b');

          export const result = imported.default;

        a.js:
          const imported = importCond('cond', './c', './d');

          export default 'module-a';

        b.js:
          const imported = importCond('cond', './c', './d');

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
    });

    // Get the generated bundle names
    let bundleNames = new Map<string, string>(
      bundleGraph
        .getBundles()
        .map((b) => [b.displayName, b.filePath.slice(distDir.length + 1)]),
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
        .toString(),
    );

    assert.deepEqual(conditionalManifest, {
      [nullthrows(bundleNames.get('a.[hash].js'))]: {
        cond: {
          ifFalseBundles: [nullthrows(bundleNames.get('d.[hash].js'))],
          ifTrueBundles: [nullthrows(bundleNames.get('c.[hash].js'))],
        },
      },
      [nullthrows(bundleNames.get('b.[hash].js'))]: {
        cond: {
          ifFalseBundles: [nullthrows(bundleNames.get('d.[hash].js'))],
          ifTrueBundles: [nullthrows(bundleNames.get('c.[hash].js'))],
        },
      },
      'index.js': {
        cond: {
          ifFalseBundles: [nullthrows(bundleNames.get('b.[hash].js'))],
          ifTrueBundles: [nullthrows(bundleNames.get('a.[hash].js'))],
        },
      },
    });
  });

  it(`should use load nested bundles when in an async bundle`, async function () {
    const workingDir = 'import-cond-false-dynamic-nested';
    const dir = path.join(__dirname, workingDir);
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
          "@atlaspack/reporter-conditional-manifest": {
            "filename": "${path.join(workingDir, 'conditional-manifest.json')}"
          }
        }

      yarn.lock: {}
      index.js:
        const conditions = { 'cond1': false, 'cond2': true };
        globalThis.__MCOND = function(key) { return conditions[key]; }

        globalThis.lazyImport = import('./lazy');

      lazy.js:
        const imported = importCond('cond1', './a', './b');

        export default imported;

      a.js:
        export default 'module-a';

      b.js:
        const imported = importCond('cond2', './c', './d');

        export default imported;

      c.js:
        export default 'module-c';

      d.js:
        export default 'module-d';
    `;

    let bundleGraph = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      outputFS: overlayFS,
      featureFlags: {
        conditionalBundlingApi: true,
      },
      defaultConfig: path.join(dir, '.parcelrc'),
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
      'module-c',
    );
  });

  it(`should have all deps as bundles in conditional manifest when same condition is used multiple times`, async function () {
    const workingDir = 'import-cond-cond-manifest-same-condition';
    const dir = path.join(__dirname, workingDir);
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
            "@atlaspack/reporter-conditional-manifest": {
              "filename": "${path.join(
                workingDir,
                'conditional-manifest.json',
              )}"
            }
          }

        yarn.lock: {}

        index.js:
          const imported1 = importCond('cond', './a', './b');
          const imported2 = importCond('cond', './c', './d');

          export const result = imported.default;

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
      featureFlags: {
        conditionalBundlingApi: true,
      },
      defaultConfig: path.join(dir, '.parcelrc'),
    });

    // Get the generated bundle names
    let bundleNames = new Map<string, string>(
      bundleGraph
        .getBundles()
        .map((b) => [b.displayName, b.filePath.slice(distDir.length + 1)]),
    );

    // Load the generated manifest
    let conditionalManifest = JSON.parse(
      overlayFS
        .readFileSync(
          path.join(distDir, workingDir, 'conditional-manifest.json'),
        )
        .toString(),
    );

    assert.deepEqual(conditionalManifest, {
      'index.js': {
        cond: {
          ifFalseBundles: [
            nullthrows(bundleNames.get('b.[hash].js')),
            nullthrows(bundleNames.get('d.[hash].js')),
          ],
          ifTrueBundles: [
            nullthrows(bundleNames.get('a.[hash].js')),
            nullthrows(bundleNames.get('c.[hash].js')),
          ],
        },
      },
    });
  });
});
