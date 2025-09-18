/**
 * A value that is computed once on read, then cached.
 *
 * @example
 *
 * ```ts
 * const lazyComputation = new LazyValue(() => doExpensiveComputation());
 *
 * const result1 = lazyComputation.get();
 * const result2 = lazyComputation.get();
 *
 * assert(result1 === result2);
 * ```
 */
export class LazyValue<T> {
  private value: T | null = null;

  constructor(private readonly factory: () => T) {}

  /**
   * Builds the value if it hasn't been built and returns it.
   */
  get(): T {
    if (this.value == null) {
      this.value = this.factory();
    }
    return this.value;
  }
}
