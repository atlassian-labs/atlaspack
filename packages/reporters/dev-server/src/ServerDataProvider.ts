/**
 * API the dev-server requires from the main application.
 */
export interface ServerDataProvider {
  getHTMLBundleFilePaths(): string[];
  requestBundle(requestedPath: string): Promise<'requested' | 'not-found'>;
}
