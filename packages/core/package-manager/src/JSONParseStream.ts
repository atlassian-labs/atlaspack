import type {JSONObject} from '@atlaspack/types';

import logger from '@atlaspack/logger';
import {Transform} from 'stream';

// Transforms chunks of json strings to parsed objects.
// Pair with split2 to parse stream of newline-delimited text.
export default class JSONParseStream extends Transform {
  constructor(options: unknown) {
    // @ts-expect-error - TS2698 - Spread types may only be created from object types.
    super({...options, objectMode: true});
  }

  // $FlowFixMe We are in object mode, so we emit objects, not strings
  _transform(
    chunk: Buffer | string,
    encoding: string,
    callback: (
      err?: Error | null | undefined,
      parsed?: JSONObject | null | undefined,
    ) => unknown,
  ) {
    try {
      let parsed;
      try {
        parsed = JSON.parse(chunk.toString());
      } catch (e: any) {
        // Be permissive and ignoreJSON parse errors in case there was
        // a non-JSON line in the package manager's stdout.
        logger.verbose({
          message: 'Ignored invalid JSON message: ' + chunk.toString(),
          origin: '@atlaspack/package-manager',
        });
        return;
      }
      callback(null, parsed);
    } catch (err: any) {
      callback(err);
    }
  }
}
