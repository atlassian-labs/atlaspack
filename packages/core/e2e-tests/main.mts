import * as reporter from 'node:test/reporters';
import {run} from 'node:test';
import * as process from 'node:process';
import {finished} from 'node:stream';

void (async function () {
  let exitCode = 0;

  const testStream = run({
    globPatterns: ['./test/**/*.test.mts'],
    concurrency: false,
    isolation: 'none',
  })
    .on('test:fail', () => {
      exitCode = 1;
    })
    .compose(new reporter.spec());

  testStream.pipe(process.stdout);
  await new Promise((res) => finished(testStream, res));
  process.exit(exitCode);
})();
