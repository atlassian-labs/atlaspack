// @flow strict-local

import type {PackageName, ConfigResult} from '@atlaspack/types';
import type {
  Config,
  InternalFileCreateInvalidation,
  InternalDevDepOptions,
} from './types';
import type {ProjectPath} from './projectPath';

import {fromProjectPathRelative} from './projectPath';
import {createEnvironment} from './Environment';
import {hashString} from '@atlaspack/rust';
import {identifierRegistry} from './IdentifierRegistry';
import type {EnvironmentRef} from './EnvironmentManager';
import {toEnvironmentId} from './EnvironmentManager';

type ConfigOpts = {|
  plugin: PackageName,
  searchPath: ProjectPath,
  isSource?: boolean,
  env?: EnvironmentRef,
  result?: ConfigResult,
  invalidateOnFileChange?: Set<ProjectPath>,
  invalidateOnConfigKeyChange?: Array<{|
    filePath: ProjectPath,
    configKey: string,
  |}>,
  invalidateOnFileCreate?: Array<InternalFileCreateInvalidation>,
  invalidateOnEnvChange?: Set<string>,
  invalidateOnOptionChange?: Set<string>,
  devDeps?: Array<InternalDevDepOptions>,
  invalidateOnStartup?: boolean,
  invalidateOnBuild?: boolean,
|};

export function createConfig({
  plugin,
  isSource,
  searchPath,
  env,
  result,
  invalidateOnFileChange,
  invalidateOnConfigKeyChange,
  invalidateOnFileCreate,
  invalidateOnEnvChange,
  invalidateOnOptionChange,
  devDeps,
  invalidateOnStartup,
  invalidateOnBuild,
}: ConfigOpts): Config {
  let environment = env ?? createEnvironment();
  const configId = hashString(
    plugin +
      fromProjectPathRelative(searchPath) +
      toEnvironmentId(environment) +
      String(isSource),
  );
  identifierRegistry.addIdentifier('config_request', configId, {
    plugin,
    searchPath,
    environmentId: toEnvironmentId(environment),
    isSource,
  });
  return {
    id: configId,
    isSource: isSource ?? false,
    searchPath,
    env: environment,
    result: result ?? null,
    cacheKey: null,
    invalidateOnFileChange: invalidateOnFileChange ?? new Set(),
    invalidateOnConfigKeyChange: invalidateOnConfigKeyChange ?? [],
    invalidateOnFileCreate: invalidateOnFileCreate ?? [],
    invalidateOnEnvChange: invalidateOnEnvChange ?? new Set(),
    invalidateOnOptionChange: invalidateOnOptionChange ?? new Set(),
    devDeps: devDeps ?? [],
    invalidateOnStartup: invalidateOnStartup ?? false,
    invalidateOnBuild: invalidateOnBuild ?? false,
  };
}
