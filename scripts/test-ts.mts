/* eslint-disable no-console */
import * as reporter from 'node:test/reporters'
import * as test from 'node:test'
import * as process from 'node:process'
import { finished } from 'node:stream'

void (async function () {
  const [,,...args] = process.argv
  const patterns = args.filter(arg => !arg.startsWith('--') || !arg.startsWith('-'))

  if (patterns.length === 0) {
    patterns.push('**/*.testn.mts')
  }

  console.table(patterns)

  let exitCode = 0
  const testStream = test.run({
    globPatterns: patterns,
    concurrency: true,
    only: !!process.env.ONLY,
    isolation: 'process',
  })
    .on('test:fail', () => {
      // @ts-ignore
      exitCode = 1
    })
    .compose(new reporter.spec())

  testStream.pipe(process.stdout)
  await new Promise((res) => finished(testStream, res))
  process.exit(exitCode)
})()
