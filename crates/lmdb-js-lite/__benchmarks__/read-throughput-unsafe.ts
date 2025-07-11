import {open as openLMDBUnsafe} from 'lmdb';
import v8 from 'node:v8';

const MAX_TIME = 10000;

// eslint-disable-next-line require-await
async function main() {
  const unsafeDB = openLMDBUnsafe({
    path: './databases/unsafe/read',
    compression: true,
    encoding: 'binary',
    eventTurnBatching: true,
  });

  const value = unsafeDB.get('benchmarkInfo');
  if (!value) throw new Error('Run prepare-read-benchmark.ts');
  const benchmarkInfo = v8.deserialize(value);
  // eslint-disable-next-line no-console
  console.log(benchmarkInfo);
  const {NUM_ENTRIES} = benchmarkInfo;

  // eslint-disable-next-line no-console
  console.log('Reading all entries out');
  {
    const start = Date.now();
    const readEntries = [];
    let i = 0;
    while (Date.now() - start < MAX_TIME) {
      readEntries.push(unsafeDB.get(String(i % NUM_ENTRIES)));
      i += 1;
    }
    const duration = Date.now() - start;
    const throughput = readEntries.length / duration;
    // eslint-disable-next-line no-console
    console.log('Unsafe Throughput:', throughput, 'entries / second');
  }
}

main().catch((err) => {
  // eslint-disable-next-line no-console
  console.error(err);
  process.exitCode = 1;
});
