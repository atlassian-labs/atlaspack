// @flow strict-local

import type {
  FilePath,
  FileCreateInvalidation,
  SourceLocation,
} from '@atlaspack/types';
import type {
  BundleGroup,
  AtlaspackOptions,
  InternalFileCreateInvalidation,
  InternalSourceLocation,
  InternalDevDepOptions,
  Invalidations,
} from './types';
import type {PackageManager} from '@atlaspack/package-manager';

import invariant from 'assert';
import baseX from 'base-x';
import {hashObject} from '@atlaspack/utils';
import {fromProjectPath, toProjectPath} from './projectPath';
import {makeConfigProxy} from './public/Config';
import {getFeatureFlag} from '@atlaspack/feature-flags';

const base62 = baseX(
  '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ',
);

export function getBundleGroupId(bundleGroup: BundleGroup): string {
  return 'bundle_group:' + bundleGroup.target.name + bundleGroup.entryAssetId;
}

export function assertSignalNotAborted(signal: ?AbortSignal): void {
  if (signal && signal.aborted) {
    throw new BuildAbortError();
  }
}

export class BuildAbortError extends Error {
  name: string = 'BuildAbortError';
}

export function getPublicId(
  id: string,
  alreadyExists: (string) => boolean,
): string {
  let encoded = base62.encode(Buffer.from(id, 'hex'));
  for (let end = 5; end <= encoded.length; end++) {
    let candidate = encoded.slice(0, end);
    if (!alreadyExists(candidate)) {
      return candidate;
    }
  }

  throw new Error('Original id was not unique');
}

// These options don't affect compilation and should cause invalidations
const ignoreOptions = new Set([
  'env', // handled by separate invalidateOnEnvChange
  'inputFS',
  'outputFS',
  'workerFarm',
  'packageManager',
  'detailedReport',
  'shouldDisableCache',
  'cacheDir',
  'shouldAutoInstall',
  'logLevel',
  'shouldProfile',
  'shouldTrace',
  'shouldPatchConsole',
  'projectRoot',
  'additionalReporters',
]);

/**
 * Creates a proxy around AtlaspackOptions to track when options are accessed.
 * This allows us to know which options are used by a specific request and invalidate
 * only the necessary work when those options change.
 *
 * When granularOptionInvalidation is enabled, uses path arrays (e.g. ['featureFlags', 'granularOptionInvalidation'])
 * for more precise invalidation. Otherwise, falls back to original string-based option tracking.
 *
 * @param {AtlaspackOptions} options - The options object to proxy
 * @param {Function} invalidateOnOptionChange - Function called with the path array when an option is accessed
 * @param {Function} [addDevDependency] - Optional function to track dev dependencies
 * @returns {AtlaspackOptions} A proxy around the options object
 */
export function optionsProxy(
  options: AtlaspackOptions,
  invalidateOnOptionChange: (path: string[] | string) => void,
  addDevDependency?: (devDep: InternalDevDepOptions) => void,
): AtlaspackOptions {
  let packageManager = addDevDependency
    ? proxyPackageManager(
        options.projectRoot,
        options.packageManager,
        addDevDependency,
      )
    : options.packageManager;

  const granularOptionInvalidationEnabled = getFeatureFlag(
    'granularOptionInvalidation',
  );

  if (granularOptionInvalidationEnabled) {
    // New behavior with granular path tracking
    // Create options object without packageManager to avoid proxying it
    // eslint-disable-next-line no-unused-vars
    const {packageManager: _packageManager, ...optionsWithoutPackageManager} =
      options;

    // Use makeConfigProxy from Config.js which is designed to track property reads
    // and provide the accessed property paths as arrays
    const proxiedOptions = makeConfigProxy((path) => {
      // Ignore specified options
      const [prop] = path;

      if (!ignoreOptions.has(prop)) {
        // Important: Always pass the full path array for granular path tracking
        // This ensures we're passing an array to invalidateOnOptionChange, not a string
        invalidateOnOptionChange(path);
      }
    }, optionsWithoutPackageManager);

    // Return the proxied options with the original or proxied packageManager
    return {
      ...proxiedOptions,
      packageManager,
    };
  } else {
    // Original behavior for backward compatibility
    return new Proxy(options, {
      get(target, prop) {
        if (prop === 'packageManager') {
          return packageManager;
        }

        if (!ignoreOptions.has(prop)) {
          // Original behavior: pass the prop as a string
          invalidateOnOptionChange(prop);
        }

        return target[prop];
      },
    });
  }
}

function proxyPackageManager(
  projectRoot: FilePath,
  packageManager: PackageManager,
  addDevDependency: (devDep: InternalDevDepOptions) => void,
): PackageManager {
  let require = (id: string, from: string, opts) => {
    addDevDependency({
      specifier: id,
      resolveFrom: toProjectPath(projectRoot, from),
      range: opts?.range,
    });
    return packageManager.require(id, from, opts);
  };

  return new Proxy(packageManager, {
    get(target, prop) {
      if (prop === 'require') {
        return require;
      }

      // $FlowFixMe
      return target[prop];
    },
  });
}

/**
 * Creates a hash value for an option to detect changes between builds.
 * Optimized to handle common types of options efficiently.
 *
 * @param {mixed} value - The option value to hash
 * @returns {string} A string hash representing the value
 */
export function hashFromOption(value: mixed): string {
  if (value == null) {
    return String(value);
  }

  if (typeof value === 'object') {
    // For all objects, use regular object hashing
    return hashObject(value);
  }

  return String(value);
}

export function invalidateOnFileCreateToInternal(
  projectRoot: FilePath,
  invalidation: FileCreateInvalidation,
): InternalFileCreateInvalidation {
  if (invalidation.glob != null) {
    return {glob: toProjectPath(projectRoot, invalidation.glob)};
  } else if (invalidation.filePath != null) {
    return {
      filePath: toProjectPath(projectRoot, invalidation.filePath),
    };
  } else {
    invariant(
      invalidation.aboveFilePath != null && invalidation.fileName != null,
    );
    return {
      fileName: invalidation.fileName,
      aboveFilePath: toProjectPath(projectRoot, invalidation.aboveFilePath),
    };
  }
}

export function createInvalidations(): Invalidations {
  return {
    invalidateOnBuild: false,
    invalidateOnStartup: false,
    invalidateOnOptionChange: new Set(),
    invalidateOnEnvChange: new Set(),
    invalidateOnFileChange: new Set(),
    invalidateOnFileCreate: [],
  };
}

export function fromInternalSourceLocation(
  projectRoot: FilePath,
  loc: ?InternalSourceLocation,
): ?SourceLocation {
  if (!loc) return loc;

  return {
    filePath: fromProjectPath(projectRoot, loc.filePath),
    start: loc.start,
    end: loc.end,
  };
}

export function toInternalSourceLocation(
  projectRoot: FilePath,
  loc: ?SourceLocation,
): ?InternalSourceLocation {
  if (!loc) return loc;

  return {
    filePath: toProjectPath(projectRoot, loc.filePath),
    start: loc.start,
    end: loc.end,
  };
}
export function toInternalSymbols<T: {|loc: ?SourceLocation|}>(
  projectRoot: FilePath,
  symbols: ?Map<Symbol, T>,
): ?Map<
  Symbol,
  {|loc: ?InternalSourceLocation, ...$Rest<T, {|loc: ?SourceLocation|}>|},
> {
  if (!symbols) return symbols;

  return new Map(
    [...symbols].map(([k, {loc, ...v}]) => [
      k,
      {
        ...v,
        loc: toInternalSourceLocation(projectRoot, loc),
      },
    ]),
  );
}
