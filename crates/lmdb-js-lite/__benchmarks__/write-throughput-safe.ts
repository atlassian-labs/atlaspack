import { randomBytes } from "node:crypto";
import { Lmdb } from "../index";
import { mkdirSync, rmSync } from "node:fs";

const KEY_SIZE = 64;
const ENTRY_SIZE = 64 * 1024; // 64KB
const MAX_TIME = 10000;
const ASYNC_WRITES = true;
const MAP_SIZE = 1024 * 1024 * 1024 * 50;
const NUM_ENTRIES = Math.floor((1024 * 1024 * 1024) / ENTRY_SIZE); // Total memory used 1GB

function generateEntry() {
  return {
    key: randomBytes(KEY_SIZE).toString(),
    value: randomBytes(ENTRY_SIZE),
  };
}

async function main() {
  {
    rmSync("./databases", {
      recursive: true,
      force: true,
    });
    mkdirSync("./databases", {
      recursive: true,
    });
    const safeDB = new Lmdb({
      path: "./databases/safe/no-batching",
      asyncWrites: ASYNC_WRITES,
      mapSize: MAP_SIZE,
    });

    {
      console.log("Generating entries for testing");
      const entries = [...Array(NUM_ENTRIES)].map(() => {
        return generateEntry();
      });
      console.log("(no-batching) Writing entries for", MAX_TIME, "ms");
      const start = Date.now();
      let numEntriesInserted = 0;
      await safeDB.startWriteTransaction();
      while (Date.now() - start < MAX_TIME) {
        const entry = entries.pop();
        if (!entry) break;
        safeDB.putNoConfirm(entry.key, entry.value);
        numEntriesInserted += 1;
      }
      await safeDB.commitWriteTransaction();
      const duration = Date.now() - start;
      const throughput = numEntriesInserted / duration;
      console.log("Throughput:", throughput, "entries / second");
    }
    safeDB.close();
  }

  {
    rmSync("./databases", {
      recursive: true,
      force: true,
    });
    mkdirSync("./databases", {
      recursive: true,
    });
    const safeDB = new Lmdb({
      path: "./databases/safe/manual",
      asyncWrites: ASYNC_WRITES,
      mapSize: MAP_SIZE,
    });
    {
      console.log("Generating entries for testing");
      const entries = [...Array(NUM_ENTRIES)].map(() => {
        return generateEntry();
      });
      console.log("(manual batching) Writing entries for", MAX_TIME, "ms");
      const start = Date.now();
      let numEntriesInserted = 0;
      let batch = [];
      while (Date.now() - start < MAX_TIME) {
        const entry = entries.pop();
        if (!entry) break;
        batch.push(entry);
        if (batch.length > 100) {
          await safeDB.putMany(batch);
          numEntriesInserted += batch.length;
          batch = [];
        }
      }
      const duration = Date.now() - start;
      const throughput = numEntriesInserted / duration;
      console.log("Safe Throughput:", throughput, "entries / second");
    }
    safeDB.close();
  }
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
