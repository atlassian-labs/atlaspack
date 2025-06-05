import type {
  InitialAtlaspackOptions,
  BuildEvent,
  BuildSuccessEvent,
  AsyncSubscription,
} from '@atlaspack/types';
import type {FarmOptions} from '@atlaspack/workers';
import type WorkerFarm from '@atlaspack/workers';

export type {default as BundleGraph} from './src/BundleGraph';
export type {default as AssetGraph} from './src/AssetGraph';
export type {
  default as RequestTracker,
  RequestGraphNode,
} from './src/RequestTracker';
export type {Node, CacheResult} from './src/types';

export class Atlaspack {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

export class Parcel {
  constructor(options: InitialAtlaspackOptions);
  run(): Promise<BuildSuccessEvent>;
  watch(
    cb?: (err: Error | null | undefined, buildEvent?: BuildEvent) => unknown,
  ): Promise<AsyncSubscription>;
}

export declare function createWorkerFarm(
  options?: Partial<FarmOptions>,
): WorkerFarm;

export default Atlaspack;
