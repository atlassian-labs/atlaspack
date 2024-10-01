// @flow strict-local

export type JsCallable<Args: $ReadOnlyArray<mixed>, Return> = (
  ...Args
) => Return;

export function jsCallable<Args: $ReadOnlyArray<mixed>, Return>(
  fn: (...Args) => Return,
): (...Args) => Return {
  return (...args: Args) => {
    try {
      return fn(...args);
    } catch (err) {
      return err;
    }
  };
}
