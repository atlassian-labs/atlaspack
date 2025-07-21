import invariant from 'assert';

export type Deferred<T> = {
  resolve(arg1: T): void;
  reject(arg1: unknown): void;
};

export function makeDeferredWithPromise<T>(): {
  deferred: Deferred<T>;
  promise: Promise<T>;
} {
  let deferred: Deferred<T> | null | undefined;
  let promise = new Promise<T>(
    (
      resolve: (result: Promise<T> | T) => void,
      reject: (error?: any) => void,
    ) => {
      deferred = {resolve, reject};
    },
  );

  // Promise constructor callback executes synchronously, so this is defined
  invariant(deferred != null);

  return {deferred, promise};
}
