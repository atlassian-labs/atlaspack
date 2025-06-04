import type {FilePath} from '@atlaspack/types';
import type {Cache} from './lib/types';
import {Readable} from 'stream';

export type {Cache} from './lib/types';

export const FSCache: {
  new (cacheDir: FilePath): Cache;
};

export class LMDBLiteCache implements Cache {
  constructor(cacheDir: FilePath);

  keys(): IterableIterator<string>;
  getBlobSync(key: string): Buffer;

  // Cache
  ensure(): Promise<void>;
  has(key: string): Promise<boolean>;
  get<T>(key: string): Promise<T | null | undefined>;
  set(key: string, value: unknown): Promise<void>;
  getStream(key: string): Readable;
  setStream(key: string, stream: Readable): Promise<void>;
  getBlob(key: string): Promise<Buffer>;
  setBlob(key: string, contents: Buffer | string): Promise<void>;
  hasLargeBlob(key: string): Promise<boolean>;
  getLargeBlob(key: string): Promise<Buffer>;
  setLargeBlob(
    key: string,
    contents: Buffer | string,
    options?: {signal?: AbortSignal},
  ): Promise<void>;
  deleteLargeBlob(key: string): Promise<void>;
  getBuffer(key: string): Promise<Buffer | null | undefined>;
  refresh(): void;
}
