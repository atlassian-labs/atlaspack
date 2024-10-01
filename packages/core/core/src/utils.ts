// @ts-expect-error - TS2307 - Cannot find module 'flow-to-typescript-codemod' or its corresponding type declarations.
import {Flow} from 'flow-to-typescript-codemod';

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

const base62 = baseX(
  '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ',
);

export function getBundleGroupId(bundleGroup: BundleGroup): string {
  return 'bundle_group:' + bundleGroup.target.name + bundleGroup.entryAssetId;
}

export function assertSignalNotAborted(signal?: AbortSignal | null): void {
  if (signal && signal.aborted) {
    throw new BuildAbortError();
  }
}

export class BuildAbortError extends Error {
  name: string = 'BuildAbortError';
}

export function getPublicId(
  id: string,
  alreadyExists: (arg1: string) => boolean,
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
  invalidateOnOptionChange: (arg1: string) => void,
  addDevDependency?: (devDep: InternalDevDepOptions) => void,
): AtlaspackOptions {
  let packageManager = addDevDependency
    ? proxyPackageManager(
        options.projectRoot,
        options.packageManager,
        addDevDependency,
      )
    : options.packageManager;
  return new Proxy(options, {
    get(target: AtlaspackOptions, prop: string) {
      if (prop === 'packageManager') {
        return packageManager;
      }

      if (!ignoreOptions.has(prop)) {
        invalidateOnOptionChange(prop);
      }

      // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type 'AtlaspackOptions'.
      return target[prop];
    },
  });
}

function proxyPackageManager(
  projectRoot: FilePath,
  packageManager: PackageManager,
  addDevDependency: (devDep: InternalDevDepOptions) => void,
): PackageManager {
  let require = (id: string, from: string, opts: any) => {
    addDevDependency({
      specifier: id,
      resolveFrom: toProjectPath(projectRoot, from),
      range: opts?.range,
    });
    return packageManager.require(id, from, opts);
  };

  return new Proxy(packageManager, {
    get(target: PackageManager, prop: string) {
      if (prop === 'require') {
        return require;
      }

      // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type 'PackageManager'.
      return target[prop];
    },
  });
}

export function hashFromOption(value: unknown): string {
  if (typeof value === 'object' && value != null) {
    // @ts-expect-error - TS2345 - Argument of type 'object' is not assignable to parameter of type '{ readonly [key: string]: unknown; }'.
    return hashObject(value);
  }

  return String(value);
}

export function invalidateOnFileCreateToInternal(
  projectRoot: FilePath,
  invalidation: FileCreateInvalidation,
): InternalFileCreateInvalidation {
  // @ts-expect-error - TS2339 - Property 'glob' does not exist on type 'FileCreateInvalidation'.
  if (invalidation.glob != null) {
    // @ts-expect-error - TS2339 - Property 'glob' does not exist on type 'FileCreateInvalidation'.
    return {glob: toProjectPath(projectRoot, invalidation.glob)};
    // @ts-expect-error - TS2339 - Property 'filePath' does not exist on type 'FileCreateInvalidation'.
  } else if (invalidation.filePath != null) {
    return {
      // @ts-expect-error - TS2339 - Property 'filePath' does not exist on type 'FileCreateInvalidation'.
      filePath: toProjectPath(projectRoot, invalidation.filePath),
    };
  } else {
    invariant(
      // @ts-expect-error - TS2339 - Property 'aboveFilePath' does not exist on type 'FileCreateInvalidation'. | TS2339 - Property 'fileName' does not exist on type 'FileCreateInvalidation'.
      invalidation.aboveFilePath != null && invalidation.fileName != null,
    );
    return {
      // @ts-expect-error - TS2339 - Property 'fileName' does not exist on type 'FileCreateInvalidation'.
      fileName: invalidation.fileName,
      // @ts-expect-error - TS2339 - Property 'aboveFilePath' does not exist on type 'FileCreateInvalidation'.
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
  loc?: InternalSourceLocation | null,
): SourceLocation | null | undefined {
  if (!loc) return loc;

  return {
    filePath: fromProjectPath(projectRoot, loc.filePath),
    start: loc.start,
    end: loc.end,
  };
}

export function toInternalSourceLocation(
  projectRoot: FilePath,
  loc?: SourceLocation | null,
): InternalSourceLocation | null | undefined {
  if (!loc) return loc;

  return {
    filePath: toProjectPath(projectRoot, loc.filePath),
    start: loc.start,
    end: loc.end,
  };
}
export function toInternalSymbols<
  T extends {
    loc: SourceLocation | null | undefined;
  },
>(
  projectRoot: FilePath,
  symbols?: Map<symbol, T> | null,
):
  | Map<
      symbol,
      {
        loc: InternalSourceLocation | null | undefined;
      } & Partial<
        Flow.Diff<
          T,
          {
            loc: SourceLocation | null | undefined;
          }
        >
      >
    >
  | null
  | undefined {
  if (!symbols) return symbols;

  return new Map(
    [...symbols].map(([k, {loc, ...v}]: [any, any]) => [
      k,
      {
        ...v,
        loc: toInternalSourceLocation(projectRoot, loc),
      },
    ]),
  );
}
