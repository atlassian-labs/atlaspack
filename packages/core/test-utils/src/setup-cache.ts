import {LMDBLiteCache} from '@atlaspack/cache';
import tempy from 'tempy';

export const cacheDir: string = tempy.directory();
export const cache: LMDBLiteCache = new LMDBLiteCache(cacheDir);
cache.ensure();
