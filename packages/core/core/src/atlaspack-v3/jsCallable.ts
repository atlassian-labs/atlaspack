export function jsCallable<Args extends ReadonlyArray<unknown>, Return>(
  fn: (...rest: Args) => Return,
): (...rest: Args) => Return {
  return (...args: Args) => {
    try {
      return fn(...args);
    } catch (err: any) {
      return err;
    }
  };
}
