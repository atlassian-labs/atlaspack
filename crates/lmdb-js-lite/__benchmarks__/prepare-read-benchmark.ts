import { randomBytes } from "node:crypto";
import { mkdirSync, rmSync } from "node:fs";
import * as v8 from "node:v8";
import { Lmdb } from "../index";

const ENTRY_SIZE = 64 * 1024; // 64KB
const ASYNC_WRITES = true;
const NUM_ENTRIES = Math.floor((1024 * 1024 * 1024 * 5) / ENTRY_SIZE); // Total memory used 1GB
const MAP_SIZE = 1024 * 1024 * 1024 * 10;

let key = 0;

function generateEntry() {
  return {
    key: String(key++),
    value: randomBytes(ENTRY_SIZE),
  };
}

async function main() {
  rmSync("./databases/safe", {
    recursive: true,
    force: true,
  });
  mkdirSync("./databases/safe", {
    recursive: true,
  });

  const safeDB = new Lmdb({
    path: "./databases/safe/read",
    asyncWrites: ASYNC_WRITES,
    mapSize: MAP_SIZE,
  });

  console.log("Generating entries for testing");
  const entries = [...Array(NUM_ENTRIES)].map(() => {
    return generateEntry();
  });
  console.log("Writing entries");
  await safeDB.startWriteTransaction();
  for (let entry of entries) {
    await safeDB.put(entry.key, entry.value);
  }
  await safeDB.put(
    "benchmarkInfo",
    v8.serialize({
      NUM_ENTRIES,
    }),
  );
  await safeDB.commitWriteTransaction();
  safeDB.close();
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
