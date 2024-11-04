// @flow strict-local

import {
  getParcelOptions,
  overlayFS,
  fsFixture,
  workerFarm,
} from '@atlaspack/test-utils';
import Atlaspack from '@atlaspack/core';

describe.only('dev dep request bug', () => {
  before(async function () {
    this.timeout(10000);
    // Warm up worker farm so that the first test doesn't account for this time.
    await workerFarm.callAllWorkers('ping', []);
  });

  it('can build an asset twice', async function () {
    this.timeout(100000);

    const i = 0;
    const entryPath = 'test/test.js';
    const options = getParcelOptions(entryPath, {
      inputFS: overlayFS,
    });

    {
      fsFixture(overlayFS)`
      test/other${i}.js:
          export default function name() {
            return 'jira ${i}';
          }
      test/test.js:
          import name from './other${i}';
          console.log('Hello, you ' + name());
      `;
      const atlaspack = new Atlaspack(options);
      await atlaspack.clearBuildCaches();
      await atlaspack.unstable_buildAssetGraph(false);
    }

    {
      fsFixture(overlayFS)`
      test/other${i}.js:
          export default function name() {
            return 'atlaspack ${i}';
          }
      test/test.js:
          import name from './other${i}';
          console.log('Hello, you ' + name());
      `;
      const atlaspack = new Atlaspack(options);
      await atlaspack.clearBuildCaches();
      await atlaspack.unstable_buildAssetGraph(false);
    }
  });
});
