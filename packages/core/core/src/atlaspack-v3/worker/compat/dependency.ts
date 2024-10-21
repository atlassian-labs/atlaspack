import type {Dependency as NapiDependency} from '@atlaspack/rust';
import type {
  Dependency as ClassicDependency,
  DependencySpecifier,
  SpecifierType,
  DependencyPriority,
  BundleBehavior,
  SourceLocation,
  Environment as ClassicEnvironment,
  Meta,
  Target as ClassicTarget,
  FilePath,
  SemverRange,
  MutableDependencySymbols as ClassicMutableDependencySymbols,
} from '@atlaspack/types';
import {Target} from './target';
import {MutableDependencySymbols} from './asset-symbols';
import {
  bundleBehaviorMap,
  specifierTypeMap,
  dependencyPriorityMap,
  packageConditionsMap,
} from './bitflags';

export class Dependency implements ClassicDependency {
  env: ClassicEnvironment;
  meta: Meta;
  target: ClassicTarget | null | undefined;
  symbols: ClassicMutableDependencySymbols;
  specifier: DependencySpecifier;
  specifierType: SpecifierType;
  priority: DependencyPriority;
  bundleBehavior: BundleBehavior | null | undefined;
  needsStableName: boolean;
  isOptional: boolean;
  isEntry: boolean;
  loc: SourceLocation | null | undefined;
  packageConditions: Array<string> | null | undefined;
  sourceAssetId: string | null | undefined;
  sourcePath: FilePath | null | undefined;
  sourceAssetType: string | null | undefined;
  resolveFrom: FilePath | null | undefined;
  range: SemverRange | null | undefined;
  pipeline: string | null | undefined;

  get id(): string {
    throw new Error('Dependency.id');
  }

  constructor(inner: NapiDependency, env: ClassicEnvironment) {
    this.env = env;
    this.meta = inner.meta || {};
    this.target = undefined;
    if (inner.target) {
      this.target = new Target(inner.target, this.env);
    }
    if (!inner.bundleBehavior) {
      this.bundleBehavior = undefined;
    } else {
      this.bundleBehavior = bundleBehaviorMap.from(inner.bundleBehavior);
    }
    this.bundleBehavior = undefined;
    this.symbols = new MutableDependencySymbols(inner.symbols || []);
    this.specifier = inner.specifier;
    this.specifierType = specifierTypeMap.from(inner.specifierType);
    this.priority = dependencyPriorityMap.from(inner.priority);
    this.needsStableName = inner.needsStableName;
    this.isOptional = inner.isOptional;
    this.isEntry = inner.isEntry;
    this.loc = inner.loc;
    this.packageConditions = packageConditionsMap.fromArray(
      inner.packageConditions || [],
    );
    this.sourceAssetId = inner.sourceAssetId;
    this.sourcePath = inner.sourcePath;
    this.sourceAssetType = inner.sourceAssetType;
    this.resolveFrom = inner.resolveFrom;
    this.range = inner.range;
    this.pipeline = inner.pipeline;
  }
}
