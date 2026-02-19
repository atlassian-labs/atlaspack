import assert from 'assert';
import path from 'path';

import {
  bundler,
  describe,
  fsFixture,
  it,
  overlayFS,
  run,
} from '@atlaspack/test-utils';

import {extractSymbolTrackerSnapshot} from './utils/symbolTracker';

async function doubleBundleForFeatureFlag(
  featureFlag: string,
  entryPath: string,
  fileSystem: typeof overlayFS,
) {
  // Options common between both builds
  let buildOptions = {
    inputFS: fileSystem,
    shouldDisableCache: true,
    mode: 'production',
    defaultTargetOptions: {
      shouldScopeHoist: true,
    },
  };

  // Build bundle graphs with the feature flag on and off
  let bOff = bundler(entryPath, {
    ...buildOptions,
    featureFlags: {[featureFlag]: false},
  });

  let bOn = bundler(entryPath, {
    ...buildOptions,
    featureFlags: {[featureFlag]: true},
  });

  let [{bundleGraph: bundleGraphOn}, {bundleGraph: bundleGraphOff}] =
    await Promise.all([bOn.run(), bOff.run()]);

  return {bundleGraphOn, bundleGraphOff};
}

async function assertSymbolsEqual(bundleGraphA, bundleGraphB) {
  await run(bundleGraphA);
  await run(bundleGraphB);

  let symbolsA = extractSymbolTrackerSnapshot(bundleGraphA);
  let symbolsB = extractSymbolTrackerSnapshot(bundleGraphB);

  assert.deepStrictEqual(
    symbolsA,
    symbolsB,
    'Expected symbol metadata to be the same',
  );
}

// This only needs to run in V3 as it's specifically testing Rust behaviour
describe.v3('rust symbol tracker parity', () => {
  it('should handle the basic case of a simple re-export', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-basic');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-basic",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {foo} from './foo';
        import {bar} from './bar';
        export default function main() {
          return foo() + bar;
        }

      foo.js:
        export function foo() { return 1; }
        export function unused() { return 999; }

      bar.js:
        export const bar = 2;
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle export renames', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-rename');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-rename",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {renamedFoo} from './barrel';
        console.log(renamedFoo());

      barrel.js:
        export {foo as renamedFoo} from './foo';

      foo.js:
        export function foo() { return 1; }
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });
});
