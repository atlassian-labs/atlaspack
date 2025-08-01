// @flow strict
import {DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';
import type {FeatureFlags} from '@atlaspack/feature-flags';

let mockedFlags: Map<string, mixed> = new Map();
let accessedFlags: Map<string, mixed> = new Map();
let randomisedFlags: Map<string, mixed> = new Map();

/**
 * @returns randomised value for consistency check feature flags
 */
function getRandomStringValue(): string {
  const values = ['NEW', 'OLD', 'NEW_AND_CHECK', 'OLD_AND_CHECK'];
  return values[Math.floor(Math.random() * values.length)];
}

/**
 * @returns randomised value for unmocked feature flags, regardless of the flag type
 */
function randomiseFlagValue(flagName: string): mixed {
  if (randomisedFlags.has(flagName)) {
    return randomisedFlags.get(flagName);
  }

  const originalValue = DEFAULT_FEATURE_FLAGS[flagName];
  let value;

  if (typeof originalValue === 'string') {
    value = getRandomStringValue();
  } else {
    value = Math.random() > 0.5;
  }
  randomisedFlags.set(flagName, value);
  return value;
}

export function setFeatureFlags(flags: $Shape<FeatureFlags>): void {
  for (const [flagName, value] of Object.entries(flags)) {
    mockedFlags.set(flagName, value);
  }
}

export function getFeatureFlagValue(flagName: string): mixed {
  let value;

  if (mockedFlags.has(flagName)) {
    value = mockedFlags.get(flagName);
  } else {
    if (process.env.RANDOM_GATES === 'true') {
      value = randomiseFlagValue(flagName);
    } else if (process.env.ALL_ENABLED === 'true') {
      // [] TODO: string value
      value = true;
    } else {
      value = DEFAULT_FEATURE_FLAGS[flagName];
    }
  }

  accessedFlags.set(flagName, value);
  return value;
}

export function getFeatureFlag(flagName: string): boolean {
  let value = getFeatureFlagValue(flagName);
  return value === true || value === 'NEW';
}

export function resetFlags(): void {
  mockedFlags.clear();
  accessedFlags.clear();
  randomisedFlags.clear();
}

export function getAccessedFlags(): Map<string, mixed> {
  return new Map(accessedFlags);
}
