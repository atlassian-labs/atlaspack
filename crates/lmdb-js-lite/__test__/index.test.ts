import { initTracingSubscriber, Lmdb } from "../index.js";
import * as v8 from "node:v8";
import * as assert from "node:assert";
import { mkdirSync, rmSync } from "node:fs";

before(() => {
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

describe("lmdb", () => {
  let db: Lmdb | null = null;
  const asyncWrites = true;
  const compression = false;
  const numEntriesToTest = 100000;
  const MAP_SIZE = 1024 * 1024 * 1024;

  it("can be opened", () => {
    db = new Lmdb({
      path: "./databases/test.db",
      asyncWrites,
      mapSize: MAP_SIZE,
    });
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
      assert.equal(resultValue, value);
    }
    {
      await db.put("key", v8.serialize({ myObject: "here", something: true }));
      const result = await db.get("key");
      const resultValue = result && v8.deserialize(result);
      assert.deepEqual(resultValue, { myObject: "here", something: true });
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
      assert.equal(resultValue, i);
    }
  });

  it('can delete entries', async () => {
    db = new Lmdb({
      path: "./databases/test.db",
      asyncWrites,
      mapSize: MAP_SIZE,
    });

    await db.put("key", v8.serialize(1));
    await db.delete("key");
    const result = await db.get("key");
    assert.equal(result, null);

    const hasEntry = db.hasSync("key");
    assert.equal(hasEntry, false);
  })

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

    it("read many entries, no transaction", async () => {
      for (let i = 0; i < numEntriesToTest; i += 1) {
        const result = await db?.get(`${i}`);
        const resultValue = result && v8.deserialize(result);
        assert.equal(resultValue, i);
      }
    });

    it("read many entries, synchronous, no transaction", async () => {
      for (let i = 0; i < numEntriesToTest; i += 1) {
        const result = db?.getSync(`${i}`);
        const resultValue = result && v8.deserialize(result);
        assert.equal(resultValue, i);
      }
    });

  });

  describe('keys', () => {
    it('can iterate over keys', async () => {
      db = new Lmdb({
        path: "./databases/keys_test.db",
        asyncWrites,
        mapSize: MAP_SIZE,
      });

      await db.put("key1", v8.serialize(1));
      await db.put("key2", v8.serialize(2));
      await db.put("key3", v8.serialize(3));

      const keys1 = await db.keysSync(0, 1);
      assert.deepEqual(keys1, ["key1"]);

      const keys2 = await db.keysSync(1, 2);
      assert.deepEqual(keys2, ["key2", "key3"]);
    });
  });
});
