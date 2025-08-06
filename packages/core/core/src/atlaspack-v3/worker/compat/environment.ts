// @ts-expect-error TS2724
import type {Environment as NapiEnvironment} from '@atlaspack/rust';
import type {
  Environment as IEnvironment,
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

export class Environment implements IEnvironment {
  id: string;
  includeNodeModules:
    | boolean
    | Array<PackageName>
    | Partial<Record<PackageName, boolean>>;
  context: EnvironmentContext;
  engines: Engines;
  outputFormat: OutputFormat;
  sourceType: SourceType;
  isLibrary: boolean;
  shouldOptimize: boolean;
  shouldScopeHoist: boolean;
  sourceMap: TargetSourceMapOptions | null | undefined;
  loc: SourceLocation | null | undefined;
  unstableSingleFileOutput: boolean;

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
    this.unstableSingleFileOutput = false;
  }

  isBrowser(): boolean {
    return (
      this.context === 'browser' ||
      this.isWorker() ||
      this.isWorklet() ||
      this.context === 'electron-renderer'
    );
  }

  isNode(): boolean {
    return this.context === 'node' || this.isElectron();
  }

  isElectron(): boolean {
    return (
      this.context === 'electron-main' || this.context === 'electron-renderer'
    );
  }

  isWorker(): boolean {
    return (
      this.context === 'web-worker' ||
      this.context === 'service-worker' ||
      this.context === 'tesseract'
    );
  }

  isWorklet(): boolean {
    return this.context === 'worklet';
  }

  isTesseract(): boolean {
    return this.context === 'tesseract';
  }

  isIsolated(): boolean {
    return this.isWorker() || this.isWorklet();
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
