import {Transform} from 'stream';

/*
 * "Taps" into the contents of a flowing stream, yielding chunks to the passed
 * callback. Continues to pass data chunks down the stream.
 */
export default class TapStream extends Transform {
  _tap: (arg1: Buffer) => unknown;
  constructor(tap: (arg1: Buffer) => unknown, options: unknown) {
    // @ts-expect-error - TS2698 - Spread types may only be created from object types.
    super({...options});
    this._tap = tap;
  }

  _transform(
    chunk: Buffer | string,
    encoding: string,
    callback: (
      err?: Error | null | undefined,
      chunk?: Buffer | string,
    ) => unknown,
  ) {
    try {
      this._tap(Buffer.from(chunk));
      callback(null, chunk);
    } catch (err: any) {
      callback(err);
    }
  }
}
