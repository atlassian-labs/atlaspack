import type {FilePath} from '@atlaspack/types';

import type {Cache} from './lib/types';

export type {Cache} from './lib/types';

export const FSCache: {
  new (cacheDir: FilePath): Cache;
};

export const LMDBLiteCache: {
  new (cacheDir: FilePath): Cache;
};
