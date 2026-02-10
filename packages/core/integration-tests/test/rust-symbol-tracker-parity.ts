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

// This only needs to run in V3 as it's specifically testing Rust behaviour
describe.v3('rust symbol tracker parity', () => {
  it('should produce identical symbol metadata when rustSymbolTracker is enabled', async function () {
    let dir = path.join(__dirname, 'rust-symbol-tracker-parity-fixture');
    await overlayFS.mkdirp(dir);

    await fsFixture(overlayFS, dir)`
      yarn.lock:
        // required for .parcelrc

      package.json:
        {
          "name": "rust-symbol-tracker-parity-fixture",
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

    // Build with legacy JS symbol propagation
    let bOff = bundler(entry, {
      inputFS: overlayFS,
      shouldDisableCache: true,
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
      featureFlags: {
        rustSymbolTracker: false,
      },
    });

    let {bundleGraph: bundleGraphOff} = await bOff.run();
    await run(bundleGraphOff);

    let symbolsOff = extractSymbolTrackerSnapshot(bundleGraphOff);

    // Build with rustSymbolTracker enabled
    let bOn = bundler(entry, {
      inputFS: overlayFS,
      shouldDisableCache: true,
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
      },
      featureFlags: {
        rustSymbolTracker: true,
      },
    });

    let {bundleGraph: bundleGraphOn} = await bOn.run();
    await run(bundleGraphOn);

    let symbolsOn = extractSymbolTrackerSnapshot(bundleGraphOn);

    assert.deepStrictEqual(
      symbolsOn,
      symbolsOff,
      'Expected rustSymbolTracker to produce the same symbol metadata as JS symbol propagation',
    );
  });
});
