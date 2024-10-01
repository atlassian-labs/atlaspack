/* eslint-disable no-console */

import assert from 'assert';
import invariant from 'assert';
import path from 'path';
import {bundle, describe, it} from '@atlaspack/test-utils';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'sinon'. '/home/ubuntu/parcel/node_modules/sinon/lib/sinon.js' implicitly has an 'any' type.
import sinon from 'sinon';

const config = path.join(
  __dirname,
  './integration/custom-configs/.parcelrc-json-reporter',
);

describe.v2('json reporter', () => {
  it('logs bundling a commonjs bundle to stdout as json', async () => {
    let consoleStub = sinon.stub(console, 'log');
    try {
      await bundle(path.join(__dirname, '/integration/commonjs/index.js'), {
        config,
        logLevel: 'info',
      });

      // @ts-expect-error - TS7006 - Parameter 'call' implicitly has an 'any' type.
      let parsedCalls = consoleStub.getCalls().map((call) => {
        invariant(typeof call.lastArg === 'string');
        return JSON.parse(call.lastArg);
      });
      for (let [iStr, parsed] of Object.entries(parsedCalls)) {
        parsed = parsed as any;
        invariant(typeof iStr === 'string');
        let i = parseInt(iStr, 10);

        if (i === 0) {
          assert.deepEqual(parsed, {type: 'buildStart'});
        } else if (i > 0 && i < 9) {
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.type, 'buildProgress');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.phase, 'transforming');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert(typeof parsed.filePath === 'string');
        } else if (i === 9) {
          assert.deepEqual(parsed, {
            type: 'buildProgress',
            phase: 'bundling',
          });
        } else if (i === 10) {
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.type, 'buildProgress');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.phase, 'packaging');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.bundleName, 'index.js');
        } else if (i === 11) {
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.type, 'buildProgress');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.phase, 'optimizing');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.bundleName, 'index.js');
        } else if (i === 12) {
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert.equal(parsed.type, 'buildSuccess');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert(typeof parsed.buildTime === 'number');
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          assert(Array.isArray(parsed.bundles));
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          let bundle = parsed.bundles[0];
          assert.equal(path.basename(bundle.filePath), 'index.js');
          assert(typeof bundle.size === 'number');
          assert(typeof bundle.time === 'number');
          assert(Array.isArray(bundle.assets));
        }
      }
    } finally {
      consoleStub.restore();
    }
  });
});
