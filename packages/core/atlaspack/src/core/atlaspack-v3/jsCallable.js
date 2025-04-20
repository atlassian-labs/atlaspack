// @flow strict-local

export function jsCallable<Args: $ReadOnlyArray<mixed>, Return>(
  fn: (...Args) => Return | Promise<Return>,
): (...Args) => Promise<Return> {
  return async (...args: Args): Promise<Return> => {
    try {
      // $FlowFixMe "await" is needed to log the error from
      // Promises and is safe to use on non-promises
      return await fn(...args);
      // $FlowFixMe
    } catch (error) {
      // eslint-disable-next-line no-console
      console.error(error);
      throw error;
    }
  };
}
