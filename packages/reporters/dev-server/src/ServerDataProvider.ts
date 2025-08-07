/**
 * API the dev-server requires from the main application.
 *
 * This interface is used to provide data to the dev-server from the main application.
 * It is used to get the list of HTML bundles and to request a bundle to be rebuilt.
 */
export interface ServerDataProvider {
  /**
   * Return relative file paths for every HTML bundle in the current build output.
   *
   * The returned paths are relative to the distribution directory on disk and
   * can therefore be used as HTTP request targets by the dev-server.
   *
   * @returns List of `.html` bundle paths. If no bundle graph is available yet,
   *          an empty array should be returned.
   */
  getHTMLBundleFilePaths(): string[];

  /**
   * Attempt to (re)build the bundle that corresponds to the given request path.
   *
   * If a bundle matching `requestedPath` exists and a request handler has been
   * provided, the implementation should trigger a rebuild and resolve the
   * promise with "requested". Otherwise the promise must resolve with
   * "not-found".
   *
   * @param requestedPath Path (relative to the distribution directory) that
   *                      identifies the bundle file requested by the browser.
   * @returns A promise that resolves to:
   *          - "requested" when the build has been successfully triggered, or
   *          - "not-found" when the bundle cannot be handled by the provider.
   */
  requestBundle(requestedPath: string): Promise<'requested' | 'not-found'>;
}
