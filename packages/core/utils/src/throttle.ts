export default function throttle<TArgs extends Iterable<unknown>>(
  // @ts-expect-error - TS2370 - A rest parameter must be of an array type.
  fn: (...args: TArgs) => unknown,
  delay: number,
  // @ts-expect-error - TS2370 - A rest parameter must be of an array type.
): (...args: TArgs) => void {
  let lastCalled: number | null | undefined;

  // @ts-expect-error - TS2370 - A rest parameter must be of an array type.
  return function throttled(...args: TArgs) {
    if (lastCalled == null || lastCalled + delay <= Date.now()) {
      // @ts-expect-error - TS2683 - 'this' implicitly has type 'any' because it does not have a type annotation.
      fn.call(this, ...args);
      lastCalled = Date.now();
    }
  };
}
