// @flow strict-local

import type {Environment} from './types';
import {addEnvironment, getEnvironment} from '@atlaspack/rust';

const localEnvCache = new Map<string, Environment>();

export function toEnvironmentId(env: Environment): string {
  addEnvironment(env);
  return env.id;
}

export function fromEnvironmentId(id: string): Environment {
  const localEnv = localEnvCache.get(id);

  if (localEnv) {
    return localEnv;
  }

  const env = Object.freeze(getEnvironment(id));
  localEnvCache.set(id, env);
  return env;
}
