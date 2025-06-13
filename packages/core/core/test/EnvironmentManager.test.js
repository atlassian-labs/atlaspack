// @flow strict-local

import assert from 'assert';
import nullthrows from 'nullthrows';
import sinon from 'sinon';
import {ATLASPACK_VERSION} from '../src/constants';
import {DEFAULT_FEATURE_FLAGS, setFeatureFlags} from '@atlaspack/feature-flags';
import {setAllEnvironments, getAllEnvironments} from '@atlaspack/rust';
import {
  loadEnvironmentsFromCache,
  writeEnvironmentsToCache,
} from '../src/EnvironmentManager';
import {DEFAULT_OPTIONS} from './test-utils';
import {LMDBLiteCache} from '@atlaspack/cache';

const options = {
  ...DEFAULT_OPTIONS,
  cache: new LMDBLiteCache(DEFAULT_OPTIONS.cacheDir),
};

describe('EnvironmentManager', () => {
  const env1 = {
    id: 'd821e85f6b50315e',
    context: 'browser',
    engines: {browsers: ['> 0.25%']},
    includeNodeModules: true,
    outputFormat: 'global',
    isLibrary: false,
    shouldOptimize: false,
    shouldScopeHoist: false,
    loc: undefined,
    sourceMap: undefined,
    sourceType: 'module',
    unstableSingleFileOutput: false,
  };
  const env2 = {
    id: 'de92f48baa8448d2',
    context: 'node',
    engines: {
      browsers: [],
      node: '>= 8',
    },
    includeNodeModules: false,
    outputFormat: 'commonjs',
    isLibrary: true,
    shouldOptimize: true,
    shouldScopeHoist: true,
    loc: null,
    sourceMap: null,
    sourceType: 'module',
    unstableSingleFileOutput: false,
  };

  beforeEach(async () => {
    await options.cache.ensure();

    for (const key of options.cache.keys()) {
      await options.cache.getNativeRef().delete(key);
    }
    setAllEnvironments([]);

    setFeatureFlags({
      ...DEFAULT_FEATURE_FLAGS,
      environmentDeduplication: true,
    });
  });

  it('should store environments by ID in the cache', async () => {
    setAllEnvironments([env1]);
    await writeEnvironmentsToCache(options.cache);

    const cachedEnv1 = await options.cache.get(
      `Environment/${ATLASPACK_VERSION}/${env1.id}`,
    );
    assert.deepEqual(cachedEnv1, env1, 'Environment 1 should be cached');
  });

  it('should list all environment IDs in the environment manager', async () => {
    const environmentIds = [env1.id, env2.id];
    setAllEnvironments([env1, env2]);
    await writeEnvironmentsToCache(options.cache);

    const cachedEnvIds = await options.cache.get(
      `EnvironmentManager/${ATLASPACK_VERSION}`,
    );
    const cachedIdsArray = nullthrows(cachedEnvIds);
    assert.equal(
      cachedIdsArray.length,
      environmentIds.length,
      'Should have same number of IDs',
    );
    assert(
      environmentIds.every((id) => cachedIdsArray.includes(id)),
      'All environment IDs should be present in cache',
    );
  });

  it('should write all environments to cache using writeEnvironmentsToCache', async () => {
    setAllEnvironments([env1, env2]);
    await writeEnvironmentsToCache(options.cache);

    // Verify each environment was stored individually
    const cachedEnv1 = await options.cache.get(
      `Environment/${ATLASPACK_VERSION}/${env1.id}`,
    );
    const cachedEnv2 = await options.cache.get(
      `Environment/${ATLASPACK_VERSION}/${env2.id}`,
    );
    assert.deepEqual(cachedEnv1, env1, 'Environment 1 should be cached');
    assert.deepEqual(cachedEnv2, env2, 'Environment 2 should be cached');

    // Verify environment IDs were stored in manager
    const cachedEnvIds = await options.cache.get(
      `EnvironmentManager/${ATLASPACK_VERSION}`,
    );
    const cachedIdsArray = nullthrows(cachedEnvIds);
    assert(
      cachedIdsArray.length === 2 &&
        [env1.id, env2.id].every((id) => cachedIdsArray.includes(id)),
      'Environment IDs should be stored in manager',
    );
  });

  it('should load environments from cache on loadRequestGraph on a subsequent build', async () => {
    // Simulate cache written on a first build
    setAllEnvironments([env1, env2]);
    await writeEnvironmentsToCache(options.cache);

    await loadEnvironmentsFromCache(options.cache);

    const loadedEnvironments = getAllEnvironments();
    assert.equal(
      loadedEnvironments.length,
      2,
      'Should load 2 environments from cache',
    );

    const env1Loaded = loadedEnvironments.find((e) => e.id === env1.id);
    const env2Loaded = loadedEnvironments.find((e) => e.id === env2.id);

    assert.deepEqual(
      env1Loaded,
      env1,
      'First environment should match cached environment',
    );
    assert.deepEqual(
      env2Loaded,
      env2,
      'Second environment should match cached environment',
    );
  });

  it('should handle empty cache gracefully without calling setAllEnvironments', async () => {
    const setAllEnvironmentsSpy = sinon.spy(setAllEnvironments);

    await assert.doesNotReject(
      loadEnvironmentsFromCache(options.cache),
      'loadEnvironmentsFromCache should not throw when cache is empty',
    );

    assert.equal(
      setAllEnvironmentsSpy.callCount,
      0,
      'setAllEnvironments should not be called when loading from empty cache',
    );
  });

  it('should not load environments from a different version', async () => {
    const setAllEnvironmentsSpy = sinon.spy(setAllEnvironments);
    const differentVersion = '2.17.2'; // A different version than ATLASPACK_VERSION

    // Store an environment with a different version
    await options.cache.set(`Environment/${differentVersion}/${env1.id}`, env1);
    await options.cache.set(`EnvironmentManager/${differentVersion}`, [
      env1.id,
    ]);

    await loadEnvironmentsFromCache(options.cache);

    assert.equal(
      setAllEnvironmentsSpy.callCount,
      0,
      'setAllEnvironments should not be called when loading from different version',
    );
    const loadedEnvironments = getAllEnvironments();
    assert.equal(
      loadedEnvironments.length,
      0,
      'Should not load any environments from different version',
    );
  });
});
