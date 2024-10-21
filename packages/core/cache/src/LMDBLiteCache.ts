import {deserialize, registerSerializableClass, serialize} from '@atlaspack/build-cache';
import {Lmdb} from '@atlaspack/rust';
import type {FilePath} from '@atlaspack/types';
import type {Cache} from './types';
import type {Readable, Writable} from 'stream';

import stream from 'stream';
import path from 'path';
import {promisify} from 'util';

import {NodeFS} from '@atlaspack/fs';

import packageJson from '../package.json';

import {FSCache} from './FSCache';

interface DBOpenOptions {
  name: string;
  // unused
  encoding: string;
  // unused
  compression: boolean;
}

export class LmdbWrapper {
  lmdb: Lmdb;

  constructor(lmdb: Lmdb) {
    this.lmdb = lmdb;

    this[Symbol.dispose] = () => {
      this.lmdb.close();
    };
  }

  get(key: string): Buffer | null {
    return this.lmdb.getSync(key);
  }

  async put(key: string, value: Buffer | string): Promise<void> {
    const buffer: Buffer =
      typeof value === 'string' ? Buffer.from(value) : value;
    await this.lmdb.put(key, buffer);
  }

  resetReadTxn() {}
}

export function open(
  directory: string,
  // eslint-disable-next-line no-unused-vars
  openOptions: DBOpenOptions,
): LmdbWrapper {
  return new LmdbWrapper(
    new Lmdb({
      path: directory,
      asyncWrites: true,
      mapSize: 1024 * 1024 * 1024 * 15,
    }),
  );
}

const pipeline: (arg1: Readable, arg2: Writable) => Promise<void> = promisify(
  stream.pipeline,
);

export class LMDBLiteCache implements Cache {
  fs: NodeFS;
  dir: FilePath;
  // $FlowFixMe
  store: any;
  fsCache: FSCache;

  constructor(cacheDir: FilePath) {
    this.fs = new NodeFS();
    this.dir = cacheDir;
    this.fsCache = new FSCache(this.fs, cacheDir);

    this.store = open(cacheDir, {
      name: 'parcel-cache',
      encoding: 'binary',
      compression: true,
    });
  }

  ensure(): Promise<void> {
    return Promise.resolve();
  }

  serialize(): {
    dir: FilePath
  } {
    return {
      dir: this.dir,
    };
  }

  static deserialize(
    opts: {
      dir: FilePath
    },
  ): LMDBLiteCache {
    return new LMDBLiteCache(opts.dir);
  }

  has(key: string): Promise<boolean> {
    return Promise.resolve(this.store.get(key) != null);
  }

  get<T>(key: string): Promise<T | null | undefined> {
    let data = this.store.get(key);
    if (data == null) {
      return Promise.resolve(null);
    }

    return Promise.resolve(deserialize(data));
  }

  async set(key: string, value: unknown): Promise<void> {
    await this.setBlob(key, serialize(value));
  }

  getStream(key: string): Readable {
    return this.fs.createReadStream(path.join(this.dir, key));
  }

  setStream(key: string, stream: Readable): Promise<void> {
    return pipeline(
      stream,
      this.fs.createWriteStream(path.join(this.dir, key)),
    );
  }

  getBlob(key: string): Promise<Buffer> {
    try {
      return Promise.resolve(this.getBlobSync(key));
    } catch (err: any) {
      return Promise.reject(err);
    }
  }

  getBlobSync(key: string): Buffer {
    const buffer = this.store.get(key);
    if (buffer == null) {
      throw new Error(`Key ${key} not found in cache`);
    }
    return buffer;
  }

  async setBlob(key: string, contents: Buffer | string): Promise<void> {
    await this.store.put(key, contents);
  }

  getBuffer(key: string): Promise<Buffer | null | undefined> {
    return Promise.resolve(this.store.get(key));
  }

  #getFilePath(key: string, index: number): string {
    return path.join(this.dir, `${key}-${index}`);
  }

  hasLargeBlob(key: string): Promise<boolean> {
    return this.fs.exists(this.#getFilePath(key, 0));
  }

  // eslint-disable-next-line require-await
  async getLargeBlob(key: string): Promise<Buffer> {
    return this.fsCache.getLargeBlob(key);
  }

  // eslint-disable-next-line require-await
  async setLargeBlob(
    key: string,
    contents: Buffer | string,
    options?: {
      signal?: AbortSignal
    },
  ): Promise<void> {
    return this.fsCache.setLargeBlob(key, contents, options);
  }

  deleteLargeBlob(key: string): Promise<void> {
    return this.fsCache.deleteLargeBlob(key);
  }

  refresh(): void {
    // Reset the read transaction for the store. This guarantees that
    // the next read will see the latest changes to the store.
    // Useful in scenarios where reads and writes are multi-threaded.
    // See https://github.com/kriszyp/lmdb-js#resetreadtxn-void
    this.store.resetReadTxn();
  }
}

registerSerializableClass(
  `${packageJson.version}:LMDBLiteCache`,
  LMDBLiteCache,
);
