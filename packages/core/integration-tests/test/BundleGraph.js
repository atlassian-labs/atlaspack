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

describe('BundleGraph', () => {
  it('can traverse assets across bundles and contexts', async () => {
    let b = await bundle(
      path.join(__dirname, '/integration/worker-shared/index.js'),
    );

    let assets = [];
    b.traverse(node => {
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

  describe.v2('getBundlesInBundleGroup', () => {
    let dir = path.join(__dirname, 'get-bundles-in-bundle-group');

    async function setupTest(): Promise<{
      bundleGraph: BundleGraph<PackagedBundle>,
      bundleGroup: BundleGroup,
    }> {
      await overlayFS.mkdirp(dir);

      await fsFixture(overlayFS, dir)`
        logo.svg:
          <svg></svg>

        index.jsx:
          import logo from 'data-url:./logo.svg';

        yarn.lock: {}
      `;

      const bundleGraph = await bundle(path.join(dir, 'index.jsx'), {
        inputFS: overlayFS,
      });
      const bundleGroup = bundleGraph.getBundleGroupsContainingBundle(
        bundleGraph.getBundles({includeInline: true})[0],
      )[0];
      return {bundleGraph, bundleGroup};
    }

    after(async () => {
      await overlayFS.rimraf(dir);
    });

    it('does not return inlineAssets by default', async () => {
      const {bundleGraph, bundleGroup} = await setupTest();
      const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup);

      assert.deepEqual(
        bundles.map(b => b.bundleBehavior),
        [null],
      );
    });

    it('does not return inlineAssets when requested', async () => {
      const {bundleGraph, bundleGroup} = await setupTest();
      const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup, {
        includeInline: false,
      });

      assert.deepEqual(
        bundles.map(b => b.bundleBehavior),
        [null],
      );
    });

    it('returns inlineAssets when requested', async () => {
      const {bundleGraph, bundleGroup} = await setupTest();
      const bundles = bundleGraph.getBundlesInBundleGroup(bundleGroup, {
        includeInline: true,
      });

      assert.deepEqual(
        bundles.map(b => b.bundleBehavior),
        [null, 'inline'],
      );
    });
  });
});
