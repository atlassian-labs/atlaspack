// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type {FilePath} from '@atlaspack/types';
// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type {Cache} from './lib/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Cache} from './lib/types';

export const FSCache: {
  new (cacheDir: FilePath): Cache;
};

export const LMDBLiteCache: {
  new (cacheDir: FilePath): Cache;
};
