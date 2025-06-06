export type ResolveFunc<T> = (value: T) => void;
export type RejectFunc = (error: unknown) => void;

export interface PromiseSubject<T = void> extends Promise<T> {
  resolve(value: T): void;
  reject(error?: any): void;
  hasSettled(): boolean;
  asPromise(): Promise<T>;
}

export type PromiseSubjectConstructor = new <T = void>() => PromiseSubject<T>;

export const PromiseSubject: PromiseSubjectConstructor = function <T = void>() {
  let _resolve: ResolveFunc<T>;
  let _reject: RejectFunc;
  let _hasSettled = false;

  const promise: any = new Promise<T>((resolve, reject) => {
    _resolve = resolve;
    _reject = reject;
  });

  promise.hasSettled = () => _hasSettled;
  promise.resolve = (v: T) => {
    _hasSettled = true;
    _resolve(v);
  };
  promise.reject = (v: any) => {
    _hasSettled = true;
    _reject(v);
  };
  promise.asPromise = () => promise.then((v: any) => v);
  return promise;
} as any;

PromiseSubject.prototype.constructor = Promise;
