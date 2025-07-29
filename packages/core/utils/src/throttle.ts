export default function throttle<TArgs extends Array<unknown>>(
  fn: (...args: TArgs) => unknown,
  delay: number,
): (...args: TArgs) => void {
  let lastCalled: number | null | undefined;

  return function throttled(this: any, ...args: TArgs) {
    if (lastCalled == null || lastCalled + delay <= Date.now()) {
      fn.call(this, ...args);
      lastCalled = Date.now();
    }
  };
}
