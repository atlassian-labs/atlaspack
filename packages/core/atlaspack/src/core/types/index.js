// @flow strict-local

import type WorkerFarm from '@atlaspack/workers';
import type {InitialAtlaspackOptionsInternal} from '../types-internal/index.js';

export type * from '../types-internal/index.js';

export type InitialAtlaspackOptions =
  InitialAtlaspackOptionsInternal<WorkerFarm>;
export type InitialParcelOptions = InitialAtlaspackOptions;
