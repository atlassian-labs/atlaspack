// @flow

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
import {Environment} from './environment';
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
  target: ?ClassicTarget;
  symbols: ClassicMutableDependencySymbols;
  #inner: NapiDependency;

  get id(): string {
    throw new Error('Dependency.id');
  }

  get specifier(): DependencySpecifier {
    return this.#inner.specifier;
  }

  get specifierType(): SpecifierType {
    return specifierTypeMap.from(this.#inner.specifierType);
  }

  get priority(): DependencyPriority {
    return dependencyPriorityMap.from(this.#inner.priority);
  }

  get bundleBehavior(): ?BundleBehavior {
    if (!this.#inner.bundleBehavior) {
      return undefined;
    }
    return bundleBehaviorMap.from(this.#inner.bundleBehavior);
  }

  get needsStableName(): boolean {
    return this.#inner.needsStableName;
  }

  get isOptional(): boolean {
    return this.#inner.isOptional;
  }

  get isEntry(): boolean {
    return this.#inner.isEntry;
  }

  get loc(): ?SourceLocation {
    return this.#inner.loc;
  }

  get packageConditions(): ?Array<string> {
    return packageConditionsMap.fromArray(this.#inner.packageConditions || []);
  }

  get sourceAssetId(): ?string {
    return this.#inner.sourceAssetId;
  }

  get sourcePath(): ?FilePath {
    return this.#inner.sourcePath;
  }

  get sourceAssetType(): ?string {
    return this.#inner.sourceAssetType;
  }

  get resolveFrom(): ?FilePath {
    return this.#inner.resolveFrom;
  }

  get range(): ?SemverRange {
    return this.#inner.range;
  }

  get pipeline(): ?string {
    return this.#inner.pipeline;
  }

  constructor(inner: NapiDependency, env: ClassicEnvironment) {
    this.#inner = inner;
    this.env = env;
    this.meta = inner.meta || {};
    this.target = undefined;
    if (inner.target) {
      this.target = new Target(inner.target, this.env);
    }
    this.symbols = new MutableDependencySymbols(inner.symbols || []);
  }
}
