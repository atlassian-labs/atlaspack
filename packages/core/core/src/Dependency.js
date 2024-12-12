// @flow
import type {
  FilePath,
  Meta,
  DependencySpecifier,
  SourceLocation,
  Symbol,
  BundleBehavior as IBundleBehavior,
  SemverRange,
} from '@atlaspack/types';
import type {Dependency, Environment, Target} from './types';
import {createDependencyId as createDependencyIdRust} from '@atlaspack/rust';
import {
  SpecifierType,
  Priority,
  BundleBehavior,
  ExportsCondition,
} from './types';

import {toInternalSourceLocation} from './utils';
import {toProjectPath} from './projectPath';
import assert from 'assert';

type DependencyOpts = {|
  id?: string,
  sourcePath?: FilePath,
  sourceAssetId?: string,
  specifier: DependencySpecifier,
  specifierType: $Keys<typeof SpecifierType>,
  priority?: $Keys<typeof Priority>,
  needsStableName?: boolean,
  bundleBehavior?: ?IBundleBehavior,
  isEntry?: boolean,
  isOptional?: boolean,
  loc?: SourceLocation,
  env: Environment,
  packageConditions?: Array<string>,
  meta?: Meta,
  resolveFrom?: FilePath,
  range?: SemverRange,
  target?: Target,
  symbols?: ?Map<
    Symbol,
    {|local: Symbol, loc: ?SourceLocation, isWeak: boolean, meta?: ?Meta|},
  >,
  pipeline?: ?string,
|};

export function createDependencyId({
  sourceAssetId,
  specifier,
  env,
  target,
  pipeline,
  specifierType,
  bundleBehavior,
  priority,
  packageConditions,
}: {|
  sourceAssetId?: string | void,
  specifier: DependencySpecifier,
  env: Environment,
  target?: Target | void,
  pipeline?: ?string,
  specifierType: $Keys<typeof SpecifierType>,
  bundleBehavior?: ?IBundleBehavior,
  priority?: $Keys<typeof Priority> | void,
  packageConditions?: Array<string> | void,
|}): string {
  assert(typeof specifierType === 'string');
  assert(typeof priority === 'string' || priority == null);
  const params = {
    sourceAssetId,
    specifier,
    environmentId: env.id,
    target,
    pipeline,
    specifierType: SpecifierType[specifierType],
    bundleBehavior,
    priority: priority ? Priority[priority] : Priority.sync,
    packageConditions,
  };
  return createDependencyIdRust(params);
}

export function createDependency(
  projectRoot: FilePath,
  opts: DependencyOpts,
): Dependency {
  let id =
    opts.id ||
    createDependencyId({
      bundleBehavior: opts.bundleBehavior,
      env: opts.env,
      packageConditions: opts.packageConditions,
      pipeline: opts.pipeline,
      priority: opts.priority,
      sourceAssetId: opts.sourceAssetId,
      specifier: opts.specifier,
      specifierType: opts.specifierType,
      target: opts.target,
    });

  let dep: Dependency = {
    id,
    specifier: opts.specifier,
    specifierType: SpecifierType[opts.specifierType],
    priority: Priority[opts.priority ?? 'sync'],
    needsStableName: opts.needsStableName ?? false,
    bundleBehavior: opts.bundleBehavior
      ? BundleBehavior[opts.bundleBehavior]
      : null,
    isEntry: opts.isEntry ?? false,
    isOptional: opts.isOptional ?? false,
    loc: toInternalSourceLocation(projectRoot, opts.loc),
    env: opts.env,
    meta: opts.meta || {},
    target: opts.target,
    sourceAssetId: opts.sourceAssetId,
    sourcePath: toProjectPath(projectRoot, opts.sourcePath),
    resolveFrom: toProjectPath(projectRoot, opts.resolveFrom),
    range: opts.range,
    symbols:
      opts.symbols &&
      new Map(
        [...opts.symbols].map(([k, v]) => [
          k,
          {
            local: v.local,
            meta: v.meta,
            isWeak: v.isWeak,
            loc: toInternalSourceLocation(projectRoot, v.loc),
          },
        ]),
      ),
    pipeline: opts.pipeline,
  };

  if (opts.packageConditions) {
    convertConditions(opts.packageConditions, dep);
  }

  return dep;
}

export function mergeDependencies(a: Dependency, b: Dependency): void {
  let {meta, symbols, needsStableName, isEntry, isOptional, ...other} = b;
  Object.assign(a, other);
  Object.assign(a.meta, meta);
  if (a.symbols && symbols) {
    for (let [k, v] of symbols) {
      a.symbols.set(k, v);
    }
  }
  if (needsStableName) a.needsStableName = true;
  if (isEntry) a.isEntry = true;
  if (!isOptional) a.isOptional = false;
}

function convertConditions(conditions: Array<string>, dep: Dependency) {
  // Store common package conditions as bit flags to reduce size.
  // Custom conditions are stored as strings.
  let packageConditions = 0;
  let customConditions = [];
  for (let condition of conditions) {
    if (ExportsCondition[condition]) {
      packageConditions |= ExportsCondition[condition];
    } else {
      customConditions.push(condition);
    }
  }

  if (packageConditions) {
    dep.packageConditions = packageConditions;
  }

  if (customConditions.length) {
    dep.customPackageConditions = customConditions;
  }
}
