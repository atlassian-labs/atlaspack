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
  /**
   * Directory where we store raw files.
   */
  cacheFilesDirectory: FilePath;

  constructor(cacheDir: FilePath) {
    this.fs = new NodeFS();
    this.dir = cacheDir;
    this.cacheFilesDirectory = path.join(cacheDir, 'files');
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
    await this.fs.mkdirp(this.cacheFilesDirectory);
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
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fs.createReadStream(path.join(this.dir, key));
    }

    return this.fs.createReadStream(this.getFileKey(key));
  }

  setStream(key: string, stream: Readable): Promise<void> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return pipeline(
        stream,
        this.fs.createWriteStream(path.join(this.dir, key)),
      );
    }

    return pipeline(stream, this.fs.createWriteStream(this.getFileKey(key)));
  }

  // eslint-disable-next-line require-await
  async getBlob(key: string): Promise<Buffer> {
    return this.getBlobSync(key);
  }

  getBlobSync(key: string): Buffer {
    // eslint-disable-next-line no-console
    console.log('getBlob', key);
    const buffer = this.store.get(key);
    if (buffer == null) {
      throw new Error(`Key ${key} not found in cache`);
    }
    return buffer;
  }

  async setBlob(key: string, contents: Buffer | string): Promise<void> {
    // eslint-disable-next-line no-console
    console.log('setBlob', key);
    await this.store.put(key, contents);
  }

  getBuffer(key: string): Promise<?Buffer> {
    return Promise.resolve(this.store.get(key));
  }

  hasLargeBlob(key: string): Promise<boolean> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.hasLargeBlob(key);
    }
    return this.fs.exists(this.getFileKey(key));
  }

  getLargeBlob(key: string): Promise<Buffer> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.getLargeBlob(key);
    }
    return this.fs.readFile(this.getFileKey(key));
  }

  async setLargeBlob(
    key: string,
    contents: Buffer | string,
    options?: {|signal?: AbortSignal|},
  ): Promise<void> {
    if (!getFeatureFlag('cachePerformanceImprovements')) {
      return this.fsCache.setLargeBlob(key, contents, options);
    }

    const targetPath = this.getFileKey(key);
    await this.fs.mkdirp(path.dirname(targetPath));
    return this.fs.writeFile(targetPath, contents);
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

  /**
   * Streams, packages are stored in files instead of LMDB.
   *
   * On this case, if a cache key happens to have a parent traversal, ../..
   * it is treated specially
   *
   * That is, something/../something and something are meant to be different
   * keys.
   *
   * Plus we do not want to store values outside of the cache directory.
   */
  getFileKey(key: string): string {
    const cleanKey = key
      .split('/')
      .map((part) => {
        if (part === '..') {
          return '$$__parent_dir$$';
        }
        return part;
      })
      .join('/');
    return path.join(this.cacheFilesDirectory, cleanKey);
  }
}

registerSerializableClass(
  `${packageJson.version}:LMDBLiteCache`,
  LMDBLiteCache,
);
