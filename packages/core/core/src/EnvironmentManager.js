// @flow strict-local
/*!
 * At the moment we're doing this change for `CoreEnvironment`,
 * but the same change must be made for `TypesEnvironment` in @atlaspack/types.
 */
import type {Environment as CoreEnvironment} from './types';
import {addEnvironment, getEnvironment} from '@atlaspack/rust';
import {getFeatureFlag} from '@atlaspack/feature-flags';

const localEnvironmentCache = new Map<string, CoreEnvironment>();

export opaque type EnvironmentId = string;
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
  localEnvironmentCache.set(id, env);
  return env;
}
