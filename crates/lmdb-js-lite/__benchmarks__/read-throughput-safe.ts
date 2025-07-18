import {Lmdb} from '../index';
import * as v8 from 'node:v8';

const MAX_TIME = 10000;
const ASYNC_WRITES = true;
const MAP_SIZE = 1024 * 1024 * 1024 * 10;

function main() {
  const safeDB = new Lmdb({
    path: './databases/safe/read',
    asyncWrites: ASYNC_WRITES,
    mapSize: MAP_SIZE,
  });

  const value = safeDB.getSync('benchmarkInfo');
  if (!value) throw new Error('Run prepare-read-benchmark.ts');
  const benchmarkInfo = v8.deserialize(value);
  // eslint-disable-next-line no-console
  console.log(benchmarkInfo);

  const {NUM_ENTRIES} = benchmarkInfo;
  // eslint-disable-next-line no-console
  console.log('(transaction) Reading all entries out');
  safeDB.startReadTransaction();
  {
    const start = Date.now();
    const readEntries = [];
    let i = 0;
    while (Date.now() - start < MAX_TIME) {
      readEntries.push(safeDB.getSync(String(i % NUM_ENTRIES)));
      i += 1;
    }
    const duration = Date.now() - start;
    const throughput = readEntries.length / duration;
    // eslint-disable-next-line no-console
    console.log(
      '(transaction) Safe Throughput:',
      throughput,
      'entries / second',
    );
  }
  safeDB.commitReadTransaction();

  // eslint-disable-next-line no-console
  console.log('(no-transaction) Reading all entries out');
  {
    const start = Date.now();
    const readEntries = [];
    let i = 0;
    while (Date.now() - start < MAX_TIME) {
      readEntries.push(safeDB.getSync(String(i % NUM_ENTRIES)));
      i += 1;
    }
    const duration = Date.now() - start;
    const throughput = readEntries.length / duration;
    // eslint-disable-next-line no-console
    console.log(
      '(no-transaction) Safe Throughput:',
      throughput,
      'entries / second',
    );
  }
}

main();