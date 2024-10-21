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
  id: string;
  includeNodeModules: boolean | Array<PackageName> | Partial<Record<PackageName, boolean>>;
  context: EnvironmentContext;
  engines: Engines;
  outputFormat: OutputFormat;
  sourceType: SourceType;
  isLibrary: boolean;
  shouldOptimize: boolean;
  shouldScopeHoist: boolean;
  sourceMap: TargetSourceMapOptions | null | undefined;
  loc: SourceLocation | null | undefined;

  constructor(inner: NapiEnvironment) {
    // TODO
    this.id = '';
    this.includeNodeModules = inner.includeNodeModules;
    this.context = inner.context;
    this.engines = inner.engines;
    this.outputFormat = inner.outputFormat;
    this.sourceType = inner.sourceType;
    this.isLibrary = inner.isLibrary;
    this.shouldOptimize = inner.shouldOptimize;
    this.shouldScopeHoist = inner.shouldScopeHoist;
    this.sourceMap = inner.sourceMap;
    this.loc = inner.loc;
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
  // eslint-disable-next-line no-unused-vars
  matchesEngines(minVersions: VersionMap, defaultValue?: boolean): boolean {
    return true;
  }

  // TODO
  // eslint-disable-next-line no-unused-vars
  supports(feature: EnvironmentFeature, defaultValue?: boolean): boolean {
    return true;
  }
}
