/*!
 * At the moment we're doing this change for `CoreEnvironment`,
 * but the same change must be made for `TypesEnvironment` in @atlaspack/types.
 */
import type {Environment as CoreEnvironment} from './types';
import {Cache} from '@atlaspack/cache';
import {
  addEnvironment,
  getEnvironment,
  getAllEnvironments,
  setAllEnvironments,
} from '@atlaspack/rust';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {instrument} from '@atlaspack/logger';
import {ATLASPACK_VERSION} from './constants';

const localEnvironmentCache = new Map<string, CoreEnvironment>();

export type EnvironmentId = string;
/**
 * When deduplication is cleaned-up this will always be a string.
 */
export type EnvironmentRef = EnvironmentId | CoreEnvironment;

/**
 * Convert environment to a ref.
 * This is what we should be using to store environments.
 */
export function toEnvironmentRef(env: CoreEnvironment): EnvironmentRef {
  if (!getFeatureFlag('environmentDeduplication')) {
    return env;
  }

  const id = toEnvironmentId(env);
  return id;
}

/**
 * Convert environment to a string ID
 */
export function toEnvironmentId(
  /**
   * Redundant type during roll-out
   */
  env: CoreEnvironment | EnvironmentRef,
): string {
  if (!getFeatureFlag('environmentDeduplication')) {
    return typeof env === 'string' ? env : env.id;
  }

  if (typeof env === 'string') {
    return env;
  }

  addEnvironment(env);
  return env.id;
}

export function fromEnvironmentId(id: EnvironmentRef): CoreEnvironment {
  if (!getFeatureFlag('environmentDeduplication')) {
    if (typeof id === 'string') {
      throw new Error(
        'This should never happen when environmentDeduplication feature-flag is off',
      );
    } else {
      return id;
    }
  }

  if (typeof id !== 'string') {
    return id;
  }

  const localEnv = localEnvironmentCache.get(id);

  if (localEnv) {
    return localEnv;
  }

  const env = Object.freeze(getEnvironment(id));
  // @ts-expect-error TS2345
  localEnvironmentCache.set(id, env);
  // @ts-expect-error TS2322
  return env;
}

/**
 * Writes all environments and their IDs to the cache
 * @param {Cache} cache
 * @returns {Promise<void>}
 */
export async function writeEnvironmentsToCache(cache: Cache): Promise<void> {
  const environments = getAllEnvironments();
  const environmentIds = new Set<string>();

  // Store each environment individually
  for (const env of environments) {
    // @ts-expect-error TS18046
    environmentIds.add(env.id);
    // @ts-expect-error TS18046
    const envKey = `Environment/${ATLASPACK_VERSION}/${env.id}`;

    await instrument(
      `RequestTracker::writeToCache::cache.put(${envKey})`,
      async () => {
        await cache.set(envKey, env);
      },
    );
  }

  // Store the list of environment IDs
  await instrument(
    `RequestTracker::writeToCache::cache.put(${`EnvironmentManager/${ATLASPACK_VERSION}`})`,
    async () => {
      await cache.set(
        `EnvironmentManager/${ATLASPACK_VERSION}`,
        Array.from(environmentIds),
      );
    },
  );
}

/**
 * Loads all environments and their IDs from the cache
 * @param {Cache} cache
 * @returns {Promise<void>}
 */
export async function loadEnvironmentsFromCache(cache: Cache): Promise<void> {
  const cachedEnvIds = await cache.get(
    `EnvironmentManager/${ATLASPACK_VERSION}`,
  );

  if (cachedEnvIds == null) {
    return;
  }

  const environments: Array<
    // @ts-expect-error TS2552
    Environment & {
      id: string;
    }
  > = [];
  // @ts-expect-error TS2488
  for (const envId of cachedEnvIds) {
    const envKey = `Environment/${ATLASPACK_VERSION}/${envId}`;
    const cachedEnv = await cache.get(envKey);
    if (cachedEnv != null) {
      environments.push(cachedEnv);
    }
  }
  if (environments.length > 0) {
    setAllEnvironments(environments);
  }
}
