// @ts-check
import { Lmdb } from "../index";

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
  }

  get(key: string) {
    return this.lmdb.getSync(key);
  }

  async put(key: string, value: Buffer | string): Promise<void> {
    if (typeof value === "string") {
      value = Buffer.from(value);
    }
    await this.lmdb.put(key, value);
  }

  resetReadTxn() {}
}

export function open(
  directory: string,
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

const defaultExport = { open };

export default defaultExport;
