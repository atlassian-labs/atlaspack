// @flow strict-local

import ThrowableDiagnostic from '@atlaspack/diagnostic';
import commander from 'commander';
import getPort from 'get-port';
import type {FileSystem} from '@atlaspack/fs';
import type {FeatureFlags} from '@atlaspack/feature-flags';
import type {InitialAtlaspackOptions, LogLevel} from '@atlaspack/types';
import {INTERNAL_ORIGINAL_CONSOLE} from '@atlaspack/logger';
import path from 'path';

function parsePort(portValue: string): number {
  let parsedPort = Number(portValue);

  // Throw an error if port value is invalid...
  if (!Number.isInteger(parsedPort)) {
    throw new Error(`Port ${portValue} is not a valid integer.`);
  }

  return parsedPort;
}

export interface Options {
  production?: boolean;
  autoinstall?: boolean;
  https?: boolean;
  cert?: string;
  key?: string;
  host: string;
  port?: string;
  hmr?: boolean;
  hmrPort?: string;
  hmrHost?: string;
  publicUrl?: string;
  detailedReport?: boolean | string;
  reporter: string[];
  trace?: boolean;
  cache?: boolean;
  cacheDir?: string;
  watchDir?: string;
  watchBackend?:
    | 'watchman'
    | 'fs-events'
    | 'inotify'
    | 'brute-force'
    | 'windows';
  watchIgnore?: string[];
  config?: string;
  logLevel?: LogLevel;
  profile?: boolean;
  contentHash?: boolean;
  featureFlag?: $Partial<FeatureFlags>;
  optimize?: boolean;
  sourceMaps?: boolean;
  scopeHoist?: boolean;
  distDir?: string;
  lazy?: string;
  lazyExclude?: string;
  target: string[];
}

// $FlowFixMe
export interface CommandExt extends commander.Command, Options {}

// $FlowFixMe
function shouldUseProductionDefaults(command: CommandExt) {
  return command.name() === 'build' || command.production === true;
}

export async function normalizeOptions(
  command: CommandExt,
  inputFS: FileSystem,
): Promise<InitialAtlaspackOptions> {
  let nodeEnv;
  if (shouldUseProductionDefaults(command)) {
    nodeEnv = process.env.NODE_ENV ?? 'production';
    // Autoinstall unless explicitly disabled or we detect a CI environment.
    command.autoinstall = !(command.autoinstall === false || process.env.CI);
  } else {
    nodeEnv = process.env.NODE_ENV ?? 'development';
  }

  // Set process.env.NODE_ENV to a default if undefined so that it is
  // available in JS configs and plugins.
  process.env.NODE_ENV = nodeEnv;

  let https = !!command.https;
  if (command.cert != null && command.key != null) {
    https = {
      cert: command.cert,
      key: command.key,
    };
  }

  let serveOptions = false;
  let {host} = command;

  // Ensure port is valid and available
  let port = parsePort(command.port != null ? String(command.port) : '1234');
  let originalPort = port;
  if (
    !shouldUseProductionDefaults(command) &&
    (command.name() === 'serve' || Boolean(command.hmr))
  ) {
    try {
      port = await getPort({port, host});
    } catch (err) {
      throw new ThrowableDiagnostic({
        diagnostic: {
          message: `Could not get available port: ${err.message}`,
          origin: 'atlaspack',
          stack: err.stack,
        },
      });
    }

    if (port !== originalPort) {
      let errorMessage = `Port "${originalPort}" could not be used`;
      if (command.port != null) {
        // Throw the error if the user defined a custom port
        throw new Error(errorMessage);
      } else {
        // Atlaspack logger is not set up at this point, so just use native INTERNAL_ORIGINAL_CONSOLE
        INTERNAL_ORIGINAL_CONSOLE.warn(errorMessage);
      }
    }
  }

  if (command.name() === 'serve') {
    let {publicUrl} = command;

    serveOptions = {
      https,
      port,
      host,
      publicUrl,
    };
  }

  let hmrOptions = null;
  if (!shouldUseProductionDefaults(command) && command.hmr !== false) {
    let hmrport = command.hmrPort != null ? parsePort(command.hmrPort) : port;
    let hmrhost = command.hmrHost != null ? String(command.hmrHost) : host;

    hmrOptions = {
      port: hmrport,
      host: hmrhost,
    };
  }

  if (command.detailedReport === true) {
    command.detailedReport = '10';
  }

  let additionalReporters = [
    {packageName: '@atlaspack/reporter-cli', resolveFrom: __filename},
    ...(command.reporter: Array<string>).map((packageName) => ({
      packageName,
      resolveFrom: path.join(inputFS.cwd(), 'index'),
    })),
  ];

  if (command.trace) {
    additionalReporters.unshift({
      packageName: '@atlaspack/reporter-tracer',
      resolveFrom: __filename,
    });
  }

  let mode = shouldUseProductionDefaults(command)
    ? 'production'
    : 'development';

  const normalizeIncludeExcludeList = (input?: string): string[] => {
    if (typeof input !== 'string') return [];
    return input.split(',').map((value) => value.trim());
  };

  return {
    shouldDisableCache: command.cache === false,
    cacheDir: command.cacheDir,
    watchDir: command.watchDir,
    watchBackend: command.watchBackend,
    watchIgnore: command.watchIgnore,
    config: command.config,
    mode,
    hmrOptions,
    shouldContentHash: hmrOptions ? false : command.contentHash,
    serveOptions,
    targets: command.target.length > 0 ? command.target : null,
    shouldAutoInstall: command.autoinstall ?? true,
    logLevel: command.logLevel,
    shouldProfile: command.profile,
    shouldTrace: command.trace,
    shouldBuildLazily: typeof command.lazy !== 'undefined',
    lazyIncludes: normalizeIncludeExcludeList(command.lazy),
    lazyExcludes: normalizeIncludeExcludeList(command.lazyExclude),
    shouldBundleIncrementally:
      process.env.ATLASPACK_INCREMENTAL_BUNDLING === 'false' ? false : true,
    detailedReport:
      command.detailedReport != null
        ? {
            assetsPerBundle: parseInt(command.detailedReport, 10),
          }
        : null,
    env: {
      NODE_ENV: nodeEnv,
    },
    additionalReporters,
    defaultTargetOptions: {
      shouldOptimize:
        command.optimize != null ? command.optimize : mode === 'production',
      sourceMaps: command.sourceMaps ?? true,
      shouldScopeHoist: command.scopeHoist,
      publicUrl: command.publicUrl,
      distDir: command.distDir,
    },
    featureFlags: command.featureFlag,
  };
}
