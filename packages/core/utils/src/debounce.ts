export default function debounce<TArgs extends Array<unknown>>(
  fn: (...args: TArgs) => unknown,
  delay: number,
): (...args: TArgs) => void {
  let timeout: NodeJS.Timeout | undefined | null;

  return function (...args: TArgs) {
    if (timeout) {
      clearTimeout(timeout);
    }

    timeout = setTimeout(() => {
      timeout = null;
      fn(...args);
    }, delay);
  };
}
