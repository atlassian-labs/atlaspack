import invariant from 'assert';
import nullthrows from 'nullthrows';
import path from 'path';
import {Reporter} from '@atlaspack/plugin';
import {Tracer} from 'chrome-trace-event';

// We need to maintain some state here to ensure we write to the same output, there should only be one
// instance of this reporter (this gets asserted below)
// @ts-expect-error - TS7034 - Variable 'tracer' implicitly has type 'any' in some locations where its type cannot be determined.
let tracer;
// @ts-expect-error - TS7034 - Variable 'writeStream' implicitly has type 'any' in some locations where its type cannot be determined.
let writeStream = null;

function millisecondsToMicroseconds(milliseconds: number) {
  return Math.floor(milliseconds * 1000);
}

// TODO: extract this to utils as it's also used in packages/core/workers/src/WorkerFarm.js
function getTimeId() {
  let now = new Date();
  return (
    String(now.getFullYear()) +
    String(now.getMonth() + 1).padStart(2, '0') +
    String(now.getDate()).padStart(2, '0') +
    '-' +
    String(now.getHours()).padStart(2, '0') +
    String(now.getMinutes()).padStart(2, '0') +
    String(now.getSeconds()).padStart(2, '0')
  );
}

export default new Reporter({
  report({event, options, logger}) {
    let filename;
    let filePath;
    switch (event.type) {
      case 'buildStart':
        // @ts-expect-error - TS7005 - Variable 'tracer' implicitly has an 'any' type.
        invariant(tracer == null, 'Tracer multiple initialisation');
        tracer = new Tracer();
        filename = `parcel-trace-${getTimeId()}.json`;
        filePath = path.join(options.projectRoot, filename);
        invariant(
          // @ts-expect-error - TS7005 - Variable 'writeStream' implicitly has an 'any' type.
          writeStream == null,
          'Trace write stream multiple initialisation',
        );
        logger.info({
          message: `Writing trace to ${filename}. See https://parceljs.org/features/profiling/#analysing-traces for more information on working with traces.`,
        });
        writeStream = options.outputFS.createWriteStream(filePath);
        nullthrows(tracer).pipe(nullthrows(writeStream));
        break;
      case 'trace':
        // Due to potential race conditions at the end of the build, we ignore any trace events that occur
        // after we've closed the write stream.
        // @ts-expect-error - TS7005 - Variable 'tracer' implicitly has an 'any' type.
        if (tracer === null) return;

        // @ts-expect-error - TS7005 - Variable 'tracer' implicitly has an 'any' type.
        tracer.completeEvent({
          name: event.name,
          cat: event.categories,
          args: event.args,
          ts: millisecondsToMicroseconds(event.ts),
          dur: millisecondsToMicroseconds(event.duration),
          tid: event.tid,
          pid: event.pid,
        });
        break;
      case 'buildSuccess':
      case 'buildFailure':
        // @ts-expect-error - TS7005 - Variable 'tracer' implicitly has an 'any' type.
        nullthrows(tracer).flush();
        tracer = null;
        // We explicitly trigger `end` on the writeStream for the trace, then we need to wait for
        // the `close` event before resolving the promise this report function returns to ensure
        // that the file has been properly closed and moved from it's temp location before Parcel
        // shuts down.
        return new Promise(
          (
            resolve: (result: Promise<undefined> | undefined) => void,
            reject: (error?: any) => void,
          ) => {
            // @ts-expect-error - TS7005 - Variable 'writeStream' implicitly has an 'any' type. | TS7006 - Parameter 'err' implicitly has an 'any' type.
            nullthrows(writeStream).once('close', (err) => {
              writeStream = null;
              if (err) {
                reject(err);
              } else {
                // @ts-expect-error - TS2794 - Expected 1 arguments, but got 0. Did you forget to include 'void' in your type argument to 'Promise'?
                resolve();
              }
            });
            // @ts-expect-error - TS7005 - Variable 'writeStream' implicitly has an 'any' type.
            nullthrows(writeStream).end();
          },
        );
    }
  },
}) as Reporter;
