import type WorkerFarm from '@atlaspack/workers';
import type {InitialAtlaspackOptionsInternal} from '@atlaspack/types-internal';

export * from '@atlaspack/types-internal';

export type InitialAtlaspackOptions =
  InitialAtlaspackOptionsInternal<WorkerFarm>;
export type InitialParcelOptions = InitialAtlaspackOptions;