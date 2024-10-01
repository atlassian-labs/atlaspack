export default function debounce<TArgs extends Array<unknown>>(
  fn: (...args: TArgs) => unknown,
  delay: number,
): (...args: TArgs) => void {
  // @ts-expect-error - TS7034 - Variable 'timeout' implicitly has type 'any' in some locations where its type cannot be determined.
  let timeout;

  return function (...args: TArgs) {
    // @ts-expect-error - TS7005 - Variable 'timeout' implicitly has an 'any' type.
    if (timeout) {
      // @ts-expect-error - TS7005 - Variable 'timeout' implicitly has an 'any' type.
      clearTimeout(timeout);
    }

    timeout = setTimeout(() => {
      timeout = null;
      fn(...args);
    }, delay);
  };
}
