import type { FilePath } from "@parcel/types";
import * as stream from "stream";
import path from "path";
import { promisify } from "util";
import {
  // @ts-expect-error missing in .d.ts
  deserialize,
  // @ts-expect-error missing in .d.ts
  registerSerializableClass,
  // @ts-expect-error missing in .d.ts
  serialize,
} from "@parcel/core";
import { type FileSystem, NodeFS } from "@parcel/fs";
import { type Cache, FSCache } from "@parcel/cache";
import { open } from "./compat";
import { Readable } from "node:stream";

const packageJson = require("../package.json");

const pipeline = promisify(stream.pipeline);

export class LMDBCacheSafe implements Cache {
  fs: FileSystem;
  dir: FilePath;
  // $FlowFixMe
  store: any;
  fsCache: Cache;

  constructor(cacheDir: FilePath) {
    this.fs = new NodeFS();
    this.dir = cacheDir;
    // @ts-expect-error The typescript bindings are wrong
    this.fsCache = new FSCache(this.fs, cacheDir);

    this.store = open(cacheDir, {
      name: "parcel-cache",
      encoding: "binary",
      compression: true,
    });
  }

  ensure(): Promise<void> {
    return Promise.resolve();
  }

  serialize(): { dir: FilePath } {
    return {
      dir: this.dir,
    };
  }

  static deserialize(opts: { dir: FilePath }): LMDBCacheSafe {
    return new LMDBCacheSafe(opts.dir);
  }

  has(key: string): Promise<boolean> {
    return Promise.resolve(this.store.get(key) != null);
  }

  get<T>(key: string): Promise<T | null> {
    const data = this.store.get(key);
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
    let buffer = this.store.get(key);
    return buffer != null
      ? Promise.resolve(buffer)
      : Promise.reject(new Error(`Key ${key} not found in cache`));
  }

  async setBlob(key: string, contents: Buffer | string): Promise<void> {
    await this.store.put(key, contents);
  }

  getBuffer(key: string): Promise<Buffer | null> {
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
    options?: { signal?: AbortSignal },
  ): Promise<void> {
    return this.fsCache.setLargeBlob(key, contents, options);
  }

  deleteLargeBlob(key: string): Promise<void> {
    // @ts-expect-error missing in .d.ts
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
  `${packageJson.version}:LMDBCacheSafe`,
  LMDBCacheSafe,
);
