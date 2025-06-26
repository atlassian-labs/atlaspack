export function jsCallable<Args extends ReadonlyArray<unknown>, Return>(fn: (...rest: Args) => Return | Promise<Return>): (...rest: Args) => Promise<Return> {
  return async (...args: Args): Promise<Return> => {
    try {
      // Promises and is safe to use on non-promises
      return await fn(...args);
    } catch (error: any) {
      // eslint-disable-next-line no-console
      console.error(error);
      throw error;
    }
  };
}
