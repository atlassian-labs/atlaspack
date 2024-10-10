// @flow

import type {Environment as NapiEnvironment} from '@atlaspack/rust';
import type {
  Environment as ClassicEnvironment,
  EnvironmentContext,
  Engines,
  PackageName,
  OutputFormat,
  SourceType,
  TargetSourceMapOptions,
  SourceLocation,
  VersionMap,
  EnvironmentFeature,
} from '@atlaspack/types';

export class Environment implements ClassicEnvironment {
  #inner: NapiEnvironment;

  // TODO
  get id(): string {
    return '';
  }

  get context(): EnvironmentContext {
    // $FlowFixMe
    return this.#inner.context;
  }

  get engines(): Engines {
    return this.#inner.engines;
  }

  get includeNodeModules():
    | boolean
    | Array<PackageName>
    | {[PackageName]: boolean, ...} {
    return this.#inner.includeNodeModules;
  }

  get outputFormat(): OutputFormat {
    // $FlowFixMe
    return this.#inner.outputFormat;
  }

  get sourceType(): SourceType {
    // $FlowFixMe
    return this.#inner.sourceType;
  }

  get isLibrary(): boolean {
    return this.#inner.isLibrary;
  }

  get shouldOptimize(): boolean {
    return this.#inner.shouldOptimize;
  }

  get shouldScopeHoist(): boolean {
    return this.#inner.shouldScopeHoist;
  }

  get sourceMap(): ?TargetSourceMapOptions {
    return this.#inner.sourceMap;
  }

  get loc(): ?SourceLocation {
    return this.#inner.loc;
  }

  constructor(inner: NapiEnvironment) {
    this.#inner = inner;
  }

  // TODO
  isBrowser(): boolean {
    return true;
  }

  // TODO
  isNode(): boolean {
    return false;
  }

  // TODO
  isElectron(): boolean {
    return false;
  }

  // TODO
  isWorker(): boolean {
    return false;
  }

  // TODO
  isWorklet(): boolean {
    return false;
  }

  // TODO
  isIsolated(): boolean {
    return false;
  }

  // TODO
  matchesEngines(minVersions: VersionMap, defaultValue?: boolean): boolean {
    return true;
  }

  // TODO
  supports(feature: EnvironmentFeature, defaultValue?: boolean): boolean {
    return true;
  }
}
