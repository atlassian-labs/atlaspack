// @flow strict-local

export type JsCallable<Args: $ReadOnlyArray<mixed>, Return> = (
  ...Args
) => JsCallableResult<Return>;

export type JsCallableResult<Return> = Promise<[string | null, Return | null]>;

export function jsCallable<Args: $ReadOnlyArray<mixed>, Return>(
  fn: (...Args) => Return | Promise<Return>,
): JsCallable<Args, Return> {
  return async (...args) => {
    try {
      const result = await fn(...args);
      return [null, result];
    } catch (err) {
      if (err instanceof Error) {
        return [err.stack, null];
      }
      // $FlowFixMe migrate to TypeScript
      let errStr = `${err}`;
      return [`${errStr}`, null];
    }
  };
}
