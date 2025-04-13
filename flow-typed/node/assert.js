// Missing flow types for node:assert
declare module "assert" {
  declare class AssertionError extends Error {}
  declare type AssertStrict = {
    (value: any, message?: string): void;
    ok(value: any, message?: string): void;
    fail(message?: string | Error): void;
    // deprecated since v10.15
    // fail(actual: any, expected: any, message: string, operator: string): void;
    equal(actual: any, expected: any, message?: string): void;
    notEqual(actual: any, expected: any, message?: string): void;
    deepEqual(actual: any, expected: any, message?: string): void;
    notDeepEqual(actual: any, expected: any, message?: string): void;
    throws(
      block: Function,
      error?: Function | RegExp | (err: any) => boolean,
      message?: string
    ): void;
    doesNotThrow(block: Function, message?: string): void;
    ifError(value: any): void;
    AssertionError: typeof AssertionError;
    strict: AssertStrict;
    ...
  }
  declare module.exports: {
    (value: any, message?: string): void,
    ok(value: any, message?: string): void,
    fail(message?: string | Error): void,
    // deprecated since v10.15
    // fail(actual: any, expected: any, message: string, operator: string): void,
    equal(actual: any, expected: any, message?: string): void,
    notEqual(actual: any, expected: any, message?: string): void,
    deepEqual(actual: any, expected: any, message?: string): void,
    notDeepEqual(actual: any, expected: any, message?: string): void,
    strictEqual(actual: any, expected: any, message?: string): void,
    notStrictEqual(actual: any, expected: any, message?: string): void,
    deepStrictEqual(actual: any, expected: any, message?: string): void,
    notDeepStrictEqual(actual: any, expected: any, message?: string): void,
    throws(
      block: Function,
      error?: Function | RegExp | (err: any) => boolean,
      message?: string
    ): void,
    doesNotThrow(block: Function, message?: string): void,
    rejects(
      block: Function,
      error?: Function | RegExp | (err: any) => boolean,
      message?: string
    ): void;
    ifError(value: any): void,
    match(actual: any, expected: any, message?: string): void,
    AssertionError: typeof AssertionError,
    strict: AssertStrict;
    ...
  }
}
