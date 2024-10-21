import assert from 'assert';
import path from 'path';
import {
  bundle,
  describe,
  fsFixture,
  overlayFS,
  inputFS,
} from '@atlaspack/test-utils';
import {loadGraphs} from '../../../dev/query/src';

describe.v2('atlaspack-query', () => {
  it('loadGraphs', async function () {
    let entries = 'index.js';
    let options = {
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: false,
      },
      shouldDisableCache: false,
      inputFS: overlayFS,
      // We need to write to the default inputFS (NodeFS) as `loadGraphs` doesn't
      // support custom FS's
      outputFS: inputFS,
      cacheDir: path.join(__dirname, '.parcel-cache'),
    };

    await fsFixture(overlayFS)`
        index.js:
            export default 1;`;

    await bundle(entries, options);

    const {assetGraph, bundleGraph, requestTracker, bundleInfo} =
      await loadGraphs(options.cacheDir);

    assert(bundleInfo, 'Could not load bundleInfo');
    assert(bundleGraph, 'Could not load bundleGraph');
    assert(assetGraph, 'Could not load assetGraph');
    assert(requestTracker, 'Count not load requestTracker');
  });
});
