import { randomBytes } from "node:crypto";
import { mkdirSync, rmSync } from "node:fs";
import * as v8 from "node:v8";
import { open as openLMDBUnsafe } from "lmdb";

const ENTRY_SIZE = 64 * 1024; // 64KB
const NUM_ENTRIES = Math.floor((1024 * 1024 * 1024 * 5) / ENTRY_SIZE); // Total memory used 1GB

let key = 0;

function generateEntry() {
  return {
    key: String(key++),
    value: randomBytes(ENTRY_SIZE),
  };
}

async function main() {
  rmSync("./databases/unsafe", {
    recursive: true,
    force: true,
  });
  mkdirSync("./databases/unsafe", {
    recursive: true,
  });

  const unsafeDB = openLMDBUnsafe({
    path: "./databases/unsafe/read",
    encoding: "binary",
    compression: true,
    eventTurnBatching: true,
  });

  console.log("Generating entries for testing");
  const entries = [...Array(NUM_ENTRIES)].map(() => {
    return generateEntry();
  });
  await unsafeDB.transaction(() => {
    console.log("Writing entries");
    for (let entry of entries) {
      unsafeDB.put(entry.key, entry.value);
    }
    unsafeDB.put(
      "benchmarkInfo",
      v8.serialize({
        NUM_ENTRIES,
      }),
    );
  });
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
