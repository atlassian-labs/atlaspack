// @flow strict-local

import {
  deserialize,
  registerSerializableClass,
  serialize,
} from '@atlaspack/build-cache';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Lmdb} from '@atlaspack/rust';
import type {FilePath} from '@atlaspack/types';
import type {Cache} from './types';
import type {Readable, Writable} from 'stream';
import fs from 'fs';
import ncp from 'ncp';
import {promisify} from 'util';
import stream from 'stream';
import path from 'path';
import {NodeFS} from '@atlaspack/fs';
// $FlowFixMe
import packageJson from '../package.json';
import {FSCache} from './FSCache';

const ncpAsync = promisify(ncp);

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

    // $FlowFixMe
    this[Symbol.dispose] = () => {
      this.lmdb.close();
    };
  }

  has(key: string): boolean {
    return this.lmdb.hasSync(key);
  }

  async delete(key: string): Promise<void> {
    await this.lmdb.delete(key);
  }

  get(key: string): Buffer | null {
    return this.lmdb.getSync(key);
  }

  async put(key: string, value: Buffer | string): Promise<void> {
    const buffer: Buffer =
      typeof value === 'string' ? Buffer.from(value) : value;
    await this.lmdb.put(key, buffer);
  }

  *keys(): Iterable<string> {
    const PAGE_SIZE = 10000000;

    let currentKeys = this.lmdb.keysSync(0, PAGE_SIZE);
    while (currentKeys.length > 0) {
      for (const key of currentKeys) {
        yield key;
      }
      currentKeys = this.lmdb.keysSync(currentKeys.length, PAGE_SIZE);
    }
  }

  compact(targetPath: string) {
    this.lmdb.compact(targetPath);
  }
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
      mapSize:
        process.env.ATLASPACK_BUILD_ENV === 'test'
          ? 1024 * 1024 * 1024
          : 1024 * 1024 * 1024 * 15,
    }),
  );
}

const pipeline: (Readable, Writable) => Promise<void> = promisify(
  stream.pipeline,
);

export type SerLMDBLiteCache = {|
  dir: FilePath,
|};

export class LMDBLiteCache implements Cache {
  fs: NodeFS;
  dir: FilePath;
  store: LmdbWrapper;
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

  /**
   * Use this to pass the native LMDB instance back to Rust.
   */
  getNativeRef(): Lmdb {
    return this.store.lmdb;
  }

  async ensure(): Promise<void> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      await this.fsCache.ensure();
    }
    return Promise.resolve();
  }

  serialize(): SerLMDBLiteCache {
    return {
      dir: this.dir,
    };
  }

  static deserialize(cache: SerLMDBLiteCache): LMDBLiteCache {
    return new LMDBLiteCache(cache.dir);
  }

  has(key: string): Promise<boolean> {
    return Promise.resolve(this.store.has(key));
  }

  get<T>(key: string): Promise<?T> {
    let data = this.store.get(key);
    if (data == null) {
      return Promise.resolve(null);
    }

    return Promise.resolve(deserialize(data));
  }

  async set(key: string, value: mixed): Promise<void> {
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

  // eslint-disable-next-line require-await
  async getBlob(key: string): Promise<Buffer> {
    return this.getBlobSync(key);
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

  getBuffer(key: string): Promise<?Buffer> {
    return Promise.resolve(this.store.get(key));
  }

  #getFilePath(key: string, index: number): string {
    return path.join(this.dir, `${key}-${index}`);
  }

  hasLargeBlob(key: string): Promise<boolean> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.hasLargeBlob(key);
    }
    return this.has(key);
  }

  /**
   * @deprecated Use getBlob instead.
   */
  getLargeBlob(key: string): Promise<Buffer> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.getLargeBlob(key);
    }
    return Promise.resolve(this.getBlobSync(key));
  }

  /**
   * @deprecated Use setBlob instead.
   */
  setLargeBlob(
    key: string,
    contents: Buffer | string,
    options?: {|signal?: AbortSignal|},
  ): Promise<void> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.setLargeBlob(key, contents, options);
    }
    return this.setBlob(key, contents);
  }

  /**
   * @deprecated Use store.delete instead.
   */
  deleteLargeBlob(key: string): Promise<void> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.deleteLargeBlob(key);
    }

    return this.store.delete(key);
  }

  keys(): Iterable<string> {
    return this.store.keys();
  }

  async compact(targetPath: string): Promise<void> {
    await fs.promises.mkdir(targetPath, {recursive: true});

    const files = await fs.promises.readdir(this.dir);
    // copy all files except data.mdb and lock.mdb to the target path (recursive)
    for (const file of files) {
      const filePath = path.join(this.dir, file);

      if (file === 'data.mdb' || file === 'lock.mdb') {
        continue;
      }

      await ncpAsync(filePath, path.join(targetPath, file));
    }

    this.store.compact(path.join(targetPath, 'data.mdb'));
  }

  refresh(): void {}
}

registerSerializableClass(
  `${packageJson.version}:LMDBLiteCache`,
  LMDBLiteCache,
);
