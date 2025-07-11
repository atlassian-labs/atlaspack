// @flow strict-local

import type {AtlaspackOptions, Target} from '../src/types';

import {DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';
import {inputFS, outputFS, cache, cacheDir} from '@atlaspack/test-utils';
import {relativePath} from '@atlaspack/utils';
import {NodePackageManager} from '@atlaspack/package-manager';
import {createEnvironment} from '../src/Environment';
import {toProjectPath} from '../src/projectPath';
import type {EnvironmentRef} from '../src/EnvironmentManager';

export const DEFAULT_OPTIONS: AtlaspackOptions = {
  cacheDir,
  parcelVersion: '',
  watchDir: __dirname,
  watchIgnore: undefined,
  watchBackend: undefined,
  entries: [],
  logLevel: 'info',
  targets: undefined,
  projectRoot: __dirname,
  shouldAutoInstall: false,
  hmrOptions: undefined,
  shouldContentHash: true,
  shouldBuildLazily: false,
  lazyIncludes: [],
  lazyExcludes: [],
  shouldBundleIncrementally: true,
  serveOptions: false,
  mode: 'development',
  env: {},
  shouldDisableCache: false,
  shouldProfile: false,
  shouldTrace: false,
  inputFS,
  outputFS,
  cache,
  shouldPatchConsole: false,
  packageManager: new NodePackageManager(inputFS, '/'),
  additionalReporters: [],
  instanceId: 'test',
  defaultTargetOptions: {
    shouldScopeHoist: false,
    shouldOptimize: false,
    publicUrl: '/',
    distDir: undefined,
    sourceMaps: false,
  },
  featureFlags: {
    ...DEFAULT_FEATURE_FLAGS,
  },
};

export const DEFAULT_ENV: EnvironmentRef = createEnvironment({
  context: 'browser',
  engines: {
    browsers: ['> 1%'],
  },
});

export const DEFAULT_TARGETS: Array<Target> = [
  {
    name: 'test',
    distDir: toProjectPath('/', '/dist'),
    distEntry: 'out.js',
    env: DEFAULT_ENV,
    publicUrl: '/',
  },
];

export function relative(f: string): string {
  return relativePath(__dirname, f, false);
}
