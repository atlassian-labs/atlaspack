// @flow strict-local

import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  fsFixture,
  it,
  overlayFS,
} from '@atlaspack/test-utils';
import type {BundleGraph, BundleGroup, PackagedBundle} from '@atlaspack/types';

const invariant = assert;

describe.only('BundleGraph', () => {
  it('can traverse assets across bundles and contexts', async () => {
    let b = await bundle(
      path.join(__dirname, '/integration/worker-shared/index.js'),
    );

    let assets = [];
    b.traverse((node) => {
      if (node.type === 'asset') {
        assets.push({
          type: node.type,
          value: path.basename(
            node.value.filePath.replace(/runtime-[0-9a-f]*/g, 'runtime'),
          ),
        });
      }
    });

    assert.deepEqual(assets, [
      {
        type: 'asset',
        value: 'index.js',
      },
      {
        type: 'asset',
        value: 'lodash.js',
      },
      {
        type: 'asset',
        value: 'worker-a.js',
      },
      {
        type: 'asset',
        value: 'lodash.js',
      },
      {
        type: 'asset',
        value: 'worker-b.js',
      },
      {
        type: 'asset',
        value: 'esmodule-helpers.js',
      },
      {
        type: 'asset',
        value: 'runtime.js',
      },
      {
        type: 'asset',
        value: 'get-worker-url.js',
      },
      {
        type: 'asset',
        value: 'bundle-url.js',
      },
      {
        type: 'asset',
        value: 'runtime.js',
      },
      {
        type: 'asset',
        value: 'get-worker-url.js',
      },
      {
        type: 'asset',
        value: 'bundle-url.js',
      },
      {
        type: 'asset',
        value: 'esmodule-helpers.js',
      },
    ]);
  });

  describe('getBundlesInBundleGroup', () => {
    let bundleGraph: BundleGraph<PackagedBundle>;
    let bundleGroup: BundleGroup;
    let dir = path.join(__dirname, 'get-bundles-in-bundle-group');

    before(async () => {
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
        logo.svg:
          <svg></svg>

        index.jsx:
          import logo from 'data-url:./logo.svg';
      `;

      bundleGraph = await bundle(path.join(dir, 'index.jsx'), {
        inputFS: overlayFS,
      });

      bundleGroup = bundleGraph.getBundleGroupsContainingBundle(
        bundleGraph.getBundles({includeInline: true})[0],
      )[0];
    });

    after(async () => {
      await overlayFS.rimraf(dir);
    });

    it('does not return inlineAssets by default', () => {
      const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup);

      assert.deepEqual(
        bundles.map((b) => b.bundleBehavior),
        [null],
      );
    });

    it('does not return inlineAssets when requested', () => {
      const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup, {
        includeInline: false,
      });
      assert.deepEqual(
        bundles.map((b) => b.bundleBehavior),
        [null],
      );
    });

    it('returns inlineAssets when requested', () => {
      const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup, {
        includeInline: true,
      });

      assert.deepEqual(
        bundles.map((b) => b.bundleBehavior),
        [null, 'inline'],
      );
    });
  });

  describe('bundle groups', () => {
    const bundlers = [
      '@atlaspack/bundler-default',
      '@atlaspack/bundler-experimental',
    ];

    bundlers.forEach((bundler) => {
      it(`${bundler} - creates a bundle group for all assets referenced in HTML`, async () => {
        await fsFixture(overlayFS, __dirname)`
        get-bundles-in-bundle-group
          index.jsx:
            console.log('hey');

          index.html:
            <script src="./index.jsx" type="module"></script>

          package.json:
            {}
          yarn.lock:
            {}

          .atlaspackrc:
            {
              "extends": "@atlaspack/config-default",
              "bundler": ${JSON.stringify(bundler)}
            }
      `;

        const bundleGraph = await bundle(
          path.join(__dirname, 'get-bundles-in-bundle-group/index.html'),
          {
            inputFS: overlayFS,
            config: '.atlaspackrc',
          },
        );

        const bundles = bundleGraph
          .getBundles({includeInline: true})
          .filter((bundle) => bundle.getMainEntry() != null);
        assert.deepEqual(
          bundles
            .map((bundle) => {
              const filePath = bundle.getMainEntry()?.filePath;
              invariant(filePath != null);
              return path.basename(filePath);
            })
            .sort(),
          ['index.html', 'index.jsx'],
        );
        assert.equal(bundles.length, 2);

        // $FlowFixMe
        const bundleGroups = bundleGraph.getAllBundleGroups();
        assert.equal(bundleGroups.length, 1);

        const indexHtmlBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('index.html'),
        );
        invariant(indexHtmlBundle != null);
        const indexHtmlBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(indexHtmlBundle);
        const indexHtmlAssetId = indexHtmlBundle.getMainEntry()?.id;
        assert.equal(indexHtmlBundleGroups.length, 1);
        assert.equal(indexHtmlBundleGroups[0].entryAssetId, indexHtmlAssetId);

        const indexBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('index.jsx'),
        );
        invariant(indexBundle != null);
        const indexBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(indexBundle);
        assert.equal(indexBundleGroups.length, 1);
        assert.equal(indexBundleGroups[0].entryAssetId, indexHtmlAssetId);
        assert.equal(indexBundleGroups[0], indexHtmlBundleGroups[0]);

        // BUNDLE REFERENCES
        const indexHtmlBundleReferences =
          bundleGraph.getReferencedBundles(indexHtmlBundle);

        assert.deepEqual(
          indexHtmlBundleReferences.map((b) =>
            path.basename(b.getMainEntry()?.filePath ?? ''),
          ),
          ['index.jsx'],
        );
      });

      it(`${bundler} - creates a bundle group per async boundary?`, async () => {
        await fsFixture(overlayFS, __dirname)`
        get-bundles-in-bundle-group
          logo.svg:
            <svg></svg>

          async.jsx:
            export default function() { return 10; }

          index.jsx:
            import logo from 'data-url:./logo.svg';
            import('./async.jsx').then(console.log);

          package.json:
            {}
          yarn.lock:
            {}

          .atlaspackrc:
            {
              "extends": "@atlaspack/config-default",
              "bundler": ${JSON.stringify(bundler)}
            }
      `;

        const bundleGraph = await bundle(
          path.join(__dirname, 'get-bundles-in-bundle-group/index.jsx'),
          {
            inputFS: overlayFS,
            config: '.atlaspackrc',
          },
        );

        const bundles = bundleGraph
          .getBundles({includeInline: true})
          .filter((bundle) => bundle.getMainEntry() != null);
        assert.deepEqual(
          bundles.map((bundle) => {
            const filePath = bundle.getMainEntry()?.filePath;
            invariant(filePath != null);
            return path.basename(filePath);
          }),
          ['index.jsx', 'async.jsx', 'logo.svg'],
        );
        assert.equal(bundles.length, 3);

        // $FlowFixMe
        const bundleGroups = bundleGraph.getAllBundleGroups();
        assert.equal(bundleGroups.length, 2);

        const indexBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('index.jsx'),
        );
        invariant(indexBundle != null);
        const indexBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(indexBundle);
        assert.equal(indexBundleGroups.length, 1);

        const svgBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('logo.svg'),
        );
        invariant(svgBundle != null);
        const svgBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(svgBundle);
        assert.equal(svgBundleGroups.length, 1);
        assert.equal(svgBundleGroups[0], indexBundleGroups[0]);

        const asyncBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('async.jsx'),
        );
        invariant(asyncBundle != null);
        const asyncBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(asyncBundle);
        assert.notEqual(asyncBundleGroups[0], indexBundleGroups[0]);
        console.log(asyncBundleGroups);
        assert.equal(asyncBundleGroups.length, 1);
      });

      it(`${bundler} - does not create bundle groups at every shared boundary`, async () => {
        await fsFixture(overlayFS, __dirname)`
        get-bundles-in-bundle-group
          logo.svg:
            <svg></svg>

          lib.js:
            export default function() { return 10; }

          async1.jsx:
            import f from './lib';
            export default function() { return f(); }

          async2.jsx:
            import f from './lib';
            export default function() { return f(); }

          index.jsx:
            import logo from 'data-url:./logo.svg';
            import('./async1.jsx').then(console.log);
            import('./async2.jsx').then(console.log);

          package.json:
            {}
          yarn.lock:
            {}

          .atlaspackrc:
            {
              "extends": "@atlaspack/config-default",
              "bundler": ${JSON.stringify(bundler)}
            }
      `;

        const bundleGraph = await bundle(
          path.join(__dirname, 'get-bundles-in-bundle-group/index.jsx'),
          {
            inputFS: overlayFS,
            config: '.atlaspackrc',
          },
        );

        // $FlowFixMe
        const bundleGroups = bundleGraph.getAllBundleGroups();
        const bundles = bundleGraph
          .getBundles({includeInline: true})
          .filter((bundle) => bundle.getMainEntry() != null);
        assert.equal(bundleGroups.length, 3);
        assert.equal(bundles.length, 4);

        const indexBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('index.jsx'),
        );
        invariant(indexBundle != null);
        const indexBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(indexBundle);
        assert.equal(indexBundleGroups.length, 1);

        const svgBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('logo.svg'),
        );
        invariant(svgBundle != null);
        const svgBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(svgBundle);
        assert.equal(svgBundleGroups.length, 1);
        assert.equal(svgBundleGroups[0], indexBundleGroups[0]);

        const asyncBundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('async1.jsx'),
        );
        invariant(asyncBundle != null);
        const asyncBundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(asyncBundle);
        assert.equal(asyncBundleGroups.length, 1);
        assert.notEqual(asyncBundleGroups[0], indexBundleGroups[0]);

        const async2Bundle = bundles.find((b) =>
          b.getMainEntry()?.filePath.includes('async2.jsx'),
        );
        invariant(async2Bundle != null);
        const async2BundleGroups =
          bundleGraph.getBundleGroupsContainingBundle(async2Bundle);
        assert.equal(async2BundleGroups.length, 1);
        assert.notEqual(async2BundleGroups[0], indexBundleGroups[0]);
        assert.notEqual(async2BundleGroups[0], asyncBundleGroups[0]);
      });
    });
  });
});
