// @flow strict-local
import type {PluginOptions, EnvMap} from '@atlaspack/types-internal';

import {NodePackageManager} from '@atlaspack/package-manager';
import {getConfig} from '../src/ConditionalManifestReporter';
import {overlayFS, fsFixture} from '@atlaspack/test-utils';
import assert from 'assert';
import {DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';

function getPluginOptions({env}: {env?: EnvMap, ...}): PluginOptions {
  return {
    mode: 'development',
    parcelVersion: 'version',
    serveOptions: false,
    shouldBuildLazily: false,
    shouldAutoInstall: false,
    logLevel: 'info',
    cacheDir: '.parcel-cache/',
    packageManager: new NodePackageManager(overlayFS, '/'),
    instanceId: 'instance-id',
    featureFlags: DEFAULT_FEATURE_FLAGS,
    detailedReport: undefined,
    hmrOptions: undefined,
    inputFS: overlayFS,
    outputFS: overlayFS,
    projectRoot: '/project-root',
    env: env ?? {},
  };
}

describe('ConditionalManifestReporter', function () {
  it('should load filename from config', async function () {
    const pluginOptions = getPluginOptions({});
    overlayFS.mkdirp(pluginOptions.projectRoot);

    await fsFixture(overlayFS, pluginOptions.projectRoot)`
      package.json:
        {
          "@atlaspack/reporter-conditional-manifest": {
            "filename": "../some-other-dir/conditional.json"
          }
        }
    `;

    const {filename} = await getConfig(pluginOptions);

    assert.equal(filename, '../some-other-dir/conditional.json');
  });

  it('should load filename from config with env vars', async function () {
    const pluginOptions = getPluginOptions({env: {USER: 'some-user'}});
    overlayFS.mkdirp(pluginOptions.projectRoot);

    await fsFixture(overlayFS, pluginOptions.projectRoot)`
      package.json:
        {
          "@atlaspack/reporter-conditional-manifest": {
            "filename": "../\${USER}/conditional.json"
          }
        }
    `;

    const {filename} = await getConfig(pluginOptions);

    assert.equal(filename, '../some-user/conditional.json');
  });
});
