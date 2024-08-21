import {randomBytes} from 'node:crypto';
import {open as openLMDBUnsafe} from 'lmdb';
import {mkdirSync, rmSync} from 'node:fs';

const KEY_SIZE = 64;
const ENTRY_SIZE = 64 * 1024; // 64KB
const MAX_TIME = 10000;
const ENABLE_COMPRESSION = true;
const NUM_ENTRIES = Math.floor((1024 * 1024 * 1024) / ENTRY_SIZE); // Total memory used 1GB

function generateEntry() {
  return {
    key: randomBytes(KEY_SIZE).toString(),
    value: randomBytes(ENTRY_SIZE),
  };
}

async function main() {
  rmSync('./databases', {
    recursive: true,
    force: true,
  });
  mkdirSync('./databases', {
    recursive: true,
  });
  const unsafeDB = openLMDBUnsafe({
    path: './databases/unsafe',
    encoding: 'binary',
    compression: ENABLE_COMPRESSION,
    eventTurnBatching: true,
  });

  console.log('Generating 1 million entries for testing');
  const entries = [...Array(NUM_ENTRIES)].map(() => {
    return generateEntry();
  });

  {
    console.log(
      'Without transaction wrapper (atlaspack usage)',
      MAX_TIME,
      'ms',
    );
    const start = Date.now();
    let numEntriesInserted = 0;
    while (Date.now() - start < MAX_TIME) {
      const entry = entries.pop();
      if (!entry) break;
      const {key, value} = entry;
      await unsafeDB.put(key, value);
      numEntriesInserted += 1;
    }
    const duration = Date.now() - start;
    const throughput = numEntriesInserted / duration;
    console.log('Throughput:', throughput, 'entries / second');
  }

  {
    console.log('Writing entries for', MAX_TIME, 'ms');
    const start = Date.now();
    let numEntriesInserted = 0;
    await unsafeDB.transaction(async () => {
      while (Date.now() - start < MAX_TIME) {
        const entry = entries.pop();
        if (!entry) break;
        const {key, value} = entry;
        await unsafeDB.put(key, value);
        numEntriesInserted += 1;
      }
    });
    const duration = Date.now() - start;
    const throughput = numEntriesInserted / duration;
    console.log('Unsafe Throughput:', throughput, 'entries / second');
  }
}

main().catch(err => {
  console.error(err);
  process.exitCode = 1;
});
