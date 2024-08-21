import { initTracingSubscriber, Lmdb } from "../index.js";
import { type Database as UnsafeDatabase, open as openLMDBUnsafe } from "lmdb";
import * as v8 from "node:v8";
import { mkdirSync, rmSync } from "node:fs";

beforeAll(() => {
  initTracingSubscriber();
});

beforeEach(() => {
  rmSync("./databases", {
    recursive: true,
    maxRetries: 10,
    force: true,
  });
  mkdirSync("./databases", {
    recursive: true,
  });
});

jest.setTimeout(40000);
describe("lmdb", () => {
  let db: Lmdb | null = null;
  const asyncWrites = true;
  const compression = false;
  const numEntriesToTest = 100000;
  const MAP_SIZE = 1024 * 1024 * 1024;

  afterEach(() => {
    db?.close();
  });

  it("can be opened", () => {
    db = new Lmdb({
      path: "./databases/test.db",
      asyncWrites,
      mapSize: MAP_SIZE,
    });
    db.close();
    db = null;
  });

  it("we can put keys and then retrieve them", async () => {
    db = new Lmdb({
      path: "./databases/test.db",
      asyncWrites,
      mapSize: MAP_SIZE,
    });

    {
      const value = Math.random().toString();
      await db.put("key", v8.serialize(value));
      const result = await db.get("key");
      const resultValue = result && v8.deserialize(result);
      expect(resultValue).toEqual(value);
    }
    {
      await db.put("key", v8.serialize({ myObject: "here", something: true }));
      const result = await db.get("key");
      const resultValue = result && v8.deserialize(result);
      expect(resultValue).toEqual({ myObject: "here", something: true });
    }
  });

  it("read and write many entries", async () => {
    db = new Lmdb({
      path: "./databases/test.db",
      asyncWrites,
      mapSize: MAP_SIZE,
    });

    const entries = [];
    for (let i = 0; i < numEntriesToTest; i += 1) {
      entries.push({
        key: `${i}`,
        value: v8.serialize(i),
      });
    }
    await db.putMany(entries);

    const values = db.getManySync(entries.map(({ key }) => key));
    for (let i = 0; i < values.length; i += 1) {
      const result = values[i];
      const resultValue = result != null && v8.deserialize(result);
      expect(resultValue).toEqual(i);
    }
  });

  describe("reading", () => {
    beforeEach(async () => {
      db = new Lmdb({
        path: "./databases/test.db",
        asyncWrites,
        mapSize: MAP_SIZE,
      });

      await db.startWriteTransaction();
      for (let i = 0; i < numEntriesToTest; i += 1) {
        await db.put(`${i}`, v8.serialize(i));
      }
      await db.commitWriteTransaction();
    });

    afterEach(() => {
      db?.close();
    });

    it("read many entries, no transaction", async () => {
      for (let i = 0; i < numEntriesToTest; i += 1) {
        const result = await db?.get(`${i}`);
        const resultValue = result && v8.deserialize(result);
        expect(resultValue).toEqual(i);
      }
    });

    it("read many entries, synchronous, no transaction", async () => {
      for (let i = 0; i < numEntriesToTest; i += 1) {
        const result = db?.getSync(`${i}`);
        const resultValue = result && v8.deserialize(result);
        expect(resultValue).toEqual(i);
      }
    });

    describe("unsafe", () => {
      let unsafeDB: UnsafeDatabase | null = null;

      beforeEach(async () => {
        unsafeDB = openLMDBUnsafe({
          path: "./databases/unsafe",
          compression,
        });

        await unsafeDB.transaction(async () => {
          for (let i = 0; i < numEntriesToTest; i += 1) {
            await unsafeDB?.put(`${i}`, v8.serialize(i));
          }
        });
      });

      it("read many entries", () => {
        for (let i = 0; i < numEntriesToTest; i += 1) {
          const result = unsafeDB?.get(`${i}`);
          const resultValue = v8.deserialize(result);
          expect(resultValue).toEqual(i);
        }
      });
    });
  });

  describe("unsafe", () => {
    it("read and write many entries", async () => {
      const unsafeDB = openLMDBUnsafe({
        path: "./databases/unsafe",
        compression,
      });

      await unsafeDB.transaction(async () => {
        for (let i = 0; i < numEntriesToTest; i += 1) {
          await unsafeDB.put(`${i}`, v8.serialize(i));
        }

        for (let i = 0; i < numEntriesToTest; i += 1) {
          const result = unsafeDB.get(`${i}`);
          const resultValue = v8.deserialize(result);
          expect(resultValue).toEqual(i);
        }
      });
    });
  });
});
