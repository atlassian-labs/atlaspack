// @flow strict-local

import {INTERNAL_ORIGINAL_CONSOLE} from '@atlaspack/logger';
import commander from 'commander';
import {DEFAULT_FEATURE_FLAGS} from '@atlaspack/feature-flags';
import type {OptionsDefinition} from './applyOptions';

// Only display choices available to callers OS
export let watcherBackendChoices: string[] = ['brute-force'];
switch (process.platform) {
  case 'darwin': {
    watcherBackendChoices.push('watchman', 'fs-events');
    break;
  }
  case 'linux': {
    watcherBackendChoices.push('watchman', 'inotify');
    break;
  }
  case 'win32': {
    watcherBackendChoices.push('watchman', 'windows');
    break;
  }
  case 'freebsd' || 'openbsd': {
    watcherBackendChoices.push('watchman');
    break;
  }
  default:
    break;
}

// --no-cache, --cache-dir, --no-source-maps, --no-autoinstall, --global?, --public-url, --log-level
// --no-content-hash, --experimental-scope-hoisting, --detailed-report
export const commonOptions: OptionsDefinition = {
  '--no-cache': 'disable the filesystem cache',
  '--config <path>':
    'specify which config to use. can be a path or a package name',
  '--cache-dir <path>': 'set the cache directory. defaults to ".parcel-cache"',
  '--watch-dir <path>':
    'set the root watch directory. defaults to nearest lockfile or source control dir.',
  '--watch-ignore [path]': [
    `list of directories watcher should not be tracking for changes. defaults to ['.git', '.hg']`,
    (dirs: string): string[] => dirs.split(','),
  ],
  '--watch-backend': new commander.Option(
    '--watch-backend <name>',
    'set watcher backend',
  ).choices(watcherBackendChoices),
  '--no-source-maps': 'disable sourcemaps',
  '--target [name]': [
    'only build given target(s)',
    (val, list) => list.concat([val]),
    [],
  ],
  '--log-level <level>': new commander.Option(
    '--log-level <level>',
    'set the log level',
  ).choices(['none', 'error', 'warn', 'info', 'verbose']),
  '--dist-dir <dir>':
    'output directory to write to when unspecified by targets',
  '--no-autoinstall': 'disable autoinstall',
  '--profile': 'enable sampling build profiling',
  '--trace': 'enable build tracing',
  '-V, --version': 'output the version number',
  '--detailed-report [count]': [
    'print the asset timings and sizes in the build report',
    parseOptionInt,
  ],
  '--reporter <name>': [
    'additional reporters to run',
    (val, acc) => {
      acc.push(val);
      return acc;
    },
    [],
  ],
  '--feature-flag <name=value>': [
    'sets the value of a feature flag',
    (value, previousValue) => {
      let [name, val] = value.split('=');
      if (name in DEFAULT_FEATURE_FLAGS) {
        let featureFlagValue;
        if (typeof DEFAULT_FEATURE_FLAGS[name] === 'boolean') {
          if (val !== 'true' && val !== 'false') {
            throw new Error(
              `Feature flag ${name} must be set to true or false`,
            );
          }
          featureFlagValue = val === 'true';
        }
        previousValue[name] = featureFlagValue ?? String(val);
      } else {
        INTERNAL_ORIGINAL_CONSOLE.warn(
          `Unknown feature flag ${name} specified, it will be ignored`,
        );
      }
      return previousValue;
    },
    {},
  ],
};

export const hmrOptions: OptionsDefinition = {
  '--no-hmr': 'disable hot module replacement',
  '-p, --port <port>': [
    'set the port to serve on. defaults to $PORT or 1234',
    process.env.PORT,
  ],
  '--host <host>':
    'set the host to listen on, defaults to listening on all interfaces',
  '--https': 'serves files over HTTPS',
  '--cert <path>': 'path to certificate to use with HTTPS',
  '--key <path>': 'path to private key to use with HTTPS',
  '--hmr-port <port>': ['hot module replacement port', process.env.HMR_PORT],
  '--hmr-host <host>': ['hot module replacement host', process.env.HMR_HOST],
};

function parseOptionInt(value: string): number {
  const parsedValue = parseInt(value, 10);
  if (isNaN(parsedValue)) {
    throw new commander.InvalidOptionArgumentError('Must be an integer.');
  }
  return parsedValue;
}
