// @flow

/** @deprecated For passing the type checks in the skipped tests */
// eslint-disable-next-line no-unused-vars
export function nextBundle(...args: any[]): any {
  throw new Error('stub');
}

/** @deprecated For passing the type checks in the skipped tests */
// eslint-disable-next-line no-unused-vars
export function assertBundleTree(...args: any[]): any {
  throw new Error('stub');
}

/** @deprecated For passing the type checks in the skipped tests */
export function deferred(): {|
  resolve: (c: any) => void,
  reject: (v: any) => void,
|} {
  throw new Error('stub');
}
