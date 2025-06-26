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
import {hashString} from '@atlaspack/rust';
import logger from '@atlaspack/logger';
import {fromProjectPath, toProjectPath} from './projectPath';
import {makeConfigProxy} from './public/Config';

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

export function optionsProxy(
  options: AtlaspackOptions,
  invalidateOnOptionChange: (path: string[]) => void,
  addDevDependency?: (devDep: InternalDevDepOptions) => void,
): AtlaspackOptions {
  let packageManager = addDevDependency
    ? proxyPackageManager(
        options.projectRoot,
        options.packageManager,
        addDevDependency,
      )
    : options.packageManager;

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
      // Always pass the full path array - this enables granular path tracking
      invalidateOnOptionChange(path);
    }
  }, optionsWithoutPackageManager);

  // Return the proxied options with the original or proxied packageManager
  return {
    ...proxiedOptions,
    packageManager,
  };
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

export function hashFromOption(value: mixed): string {
  if (value == null) {
    return String(value);
  }

  if (typeof value === 'object') {
    // For arrays, only hash the length and a sample of elements
    if (Array.isArray(value)) {
      if (value.length > 100) {
        // For large arrays, just hash the length and sample a few elements
        return hashString(
          `array:${value.length}:${String(value[0])}:${String(
            value[Math.floor(value.length / 2)],
          )}:${String(value[value.length - 1])}`,
        );
      }
    }

    // For objects with conditionalBundlingApi, use a simplified hash
    if (
      typeof value === 'object' &&
      value !== null &&
      'conditionalBundlingApi' in value
    ) {
      const api = value.conditionalBundlingApi;
      // Create a simplified representation with just the essential properties
      return hashString(
        `conditionalBundlingApi:${api != null ? 'true' : 'false'}:${
          api != null && typeof api === 'object' ? Object.keys(api).length : 0
        }`,
      );
    }

    // For other objects, use regular object hashing
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

/**
 * Throttling mechanism to prevent excessive resource consumption
 * during option invalidation.
 */
export class ResourceThrottler {
  static instance: ?ResourceThrottler;
  lastCleanupTime: number = 0;
  processingCount: number = 0;
  maxConcurrent: number = 10;
  cleanupInterval: number = 30000; // 30 seconds

  static getInstance(): ResourceThrottler {
    if (!ResourceThrottler.instance) {
      ResourceThrottler.instance = new ResourceThrottler();
    }
    return ResourceThrottler.instance;
  }

  /**
   * Check if we should throttle processing to avoid resource exhaustion
   */
  shouldThrottle(): boolean {
    const now = Date.now();

    // If we're over the concurrent limit
    if (this.processingCount >= this.maxConcurrent) {
      return true;
    }

    // Periodically reset the counter to recover from leaks
    if (now - this.lastCleanupTime > this.cleanupInterval) {
      this.processingCount = 0;
      this.lastCleanupTime = now;
    }

    return false;
  }

  /**
   * Increment the processing counter
   */
  startProcessing(): void {
    this.processingCount++;
  }

  /**
   * Decrement the processing counter
   */
  endProcessing(): void {
    this.processingCount = Math.max(0, this.processingCount - 1);
  }

  /**
   * Run an operation with throttling
   * @param {Function} operation - The operation to run
   * @returns {Promise<T>} - The result of the operation
   */
  async runWithThrottling<T>(operation: () => Promise<T>): Promise<T> {
    if (this.shouldThrottle()) {
      logger.verbose({
        origin: '@atlaspack/core',
        message: 'Throttling operation due to resource constraints',
        meta: {
          processingCount: this.processingCount,
          maxConcurrent: this.maxConcurrent,
          trackableEvent: 'resource_throttling',
        },
      });

      // Wait a bit before trying again
      await new Promise((resolve) => setTimeout(resolve, 100));
      return this.runWithThrottling(operation);
    }

    this.startProcessing();
    try {
      return await operation();
    } finally {
      this.endProcessing();
    }
  }
}
