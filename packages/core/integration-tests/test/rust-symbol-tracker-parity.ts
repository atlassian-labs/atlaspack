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

  it('should handle chained renames through multiple barrel files', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-chained');
    await overlayFS.mkdirp(dir);

    // index.js imports "finalName" from barrel1
    // barrel1 re-exports "middleName" as "finalName" from barrel2
    // barrel2 re-exports "originalName" as "middleName" from source
    // source exports "originalName"
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-chained",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {finalName} from './barrel1';
        console.log(finalName());

      barrel1.js:
        export {middleName as finalName} from './barrel2';

      barrel2.js:
        export {originalName as middleName} from './source';

      source.js:
        export function originalName() { return 42; }
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle star re-exports', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-star');
    await overlayFS.mkdirp(dir);

    // index.js imports specific symbols from barrel
    // barrel re-exports everything from multiple source files via export *
    // Only the used symbols should propagate through
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-star",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {foo, bar} from './barrel';
        console.log(foo() + bar);

      barrel.js:
        export * from './foo';
        export * from './bar';

      foo.js:
        export function foo() { return 1; }
        export function unusedFoo() { return 999; }

      bar.js:
        export const bar = 2;
        export const unusedBar = 888;
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle chained star re-exports', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-chained-star');
    await overlayFS.mkdirp(dir);

    // index.js imports specific symbols from barrel
    // barrel re-exports everything from multiple source files via export *
    // Only the used symbols should propagate through
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-star",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {foo, bar} from './barrel';
        console.log(foo() + bar);

      barrel.js:
        export * from './foo';
        export * from './bar';

      foo.js:
        export * from './sub-foo';
        export function topLevelFoo() { return 5; }


      sub-foo.js:
        export function foo() { return 1; }
        export function unusedFoo() { return 999; }

      bar.js:
        export const bar = 2;
        export const unusedBar = 888;
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle diamond pattern star re-exports', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-diamond');
    await overlayFS.mkdirp(dir);

    // Diamond pattern:
    //       index.js
    //          ↓
    //       barrel.js (export * from left, export * from right)
    //        ↙    ↘
    //   left.js   right.js (both: export * from shared)
    //        ↘    ↙
    //       shared.js (exports foo)
    //
    // The symbol 'foo' can be reached via two paths:
    // barrel -> left -> shared
    // barrel -> right -> shared
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-diamond",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {foo} from './barrel';
        console.log(foo);

      barrel.js:
        export * from './left';
        export * from './right';

      left.js:
        export * from './shared';
        export const leftOnly = 'left';

      right.js:
        export * from './shared';
        export const rightOnly = 'right';

      shared.js:
        export const foo = 'shared-foo';
        export const unusedShared = 'unused';
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  // Namespace re-export tests: `export * as ns from './dep'`

  it('should handle basic namespace re-export', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-ns-basic');
    await overlayFS.mkdirp(dir);

    // barrel.js: export * as ns from './dep'
    // index.js: import { ns } from './barrel'; console.log(ns.foo);
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-ns-basic",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {ns} from './barrel';
        console.log(ns.foo, ns.bar);

      barrel.js:
        export * as ns from './dep';

      dep.js:
        export function foo() { return 1; }
        export function bar() { return 2; }
        export function unused() { return 999; }
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle namespace re-export alongside named exports', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-ns-mixed');
    await overlayFS.mkdirp(dir);

    // barrel.js has both a namespace re-export and its own named export
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-ns-mixed",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {ns, localFn} from './barrel';
        console.log(ns.foo, localFn());

      barrel.js:
        export * as ns from './dep';
        export function localFn() { return 42; }

      dep.js:
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

  it('should handle namespace re-export alongside star re-export', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-ns-with-star');
    await overlayFS.mkdirp(dir);

    // barrel.js has both:
    //   export * from './star-dep';     (star re-export)
    //   export * as ns from './ns-dep'; (namespace re-export)
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-ns-with-star",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {foo, ns} from './barrel';
        console.log(foo, ns.bar);

      barrel.js:
        export * from './star-dep';
        export * as ns from './ns-dep';

      star-dep.js:
        export function foo() { return 1; }
        export function unusedFoo() { return 999; }

      ns-dep.js:
        export function bar() { return 2; }
        export function unusedBar() { return 888; }
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle multiple namespace re-exports from same barrel', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-ns-multi');
    await overlayFS.mkdirp(dir);

    // barrel.js:
    //   export * as nsFoo from './foo';
    //   export * as nsBar from './bar';
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-ns-multi",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {nsFoo, nsBar} from './barrel';
        console.log(nsFoo.a, nsBar.b);

      barrel.js:
        export * as nsFoo from './foo';
        export * as nsBar from './bar';

      foo.js:
        export const a = 1;
        export const unusedA = 999;

      bar.js:
        export const b = 2;
        export const unusedB = 888;
    `;

    let entry = path.join(dir, 'index.js');

    let {bundleGraphOn, bundleGraphOff} = await doubleBundleForFeatureFlag(
      'rustSymbolTracker',
      entry,
      overlayFS,
    );

    await assertSymbolsEqual(bundleGraphOn, bundleGraphOff);
  });

  it('should handle chained namespace re-exports', async () => {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-ns-chained');
    await overlayFS.mkdirp(dir);

    // barrel1.js: export * as innerNs from './barrel2';
    // barrel2.js: export * as deepNs from './source';
    // source.js: export function foo() {}
    //
    // index.js: import { innerNs } from './barrel1';
    // innerNs.deepNs.foo()
    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-ns-chained",
          "sideEffects": false,
          "version": "1.0.0"
        }

      index.js:
        import {innerNs} from './barrel1';
        console.log(innerNs.deepNs.foo());

      barrel1.js:
        export * as innerNs from './barrel2';

      barrel2.js:
        export * as deepNs from './source';

      source.js:
        export function foo() { return 42; }
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
