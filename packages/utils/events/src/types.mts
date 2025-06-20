export interface IDisposable {
  /** This can return a Promise, as dispose() of all inner disposables are
   * awaited in Disposable#dispose()
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  dispose(): (any | Promise<any>);
}
