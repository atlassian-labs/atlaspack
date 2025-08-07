import type {
  BundleGraph,
  PackagedBundle,
  BuildSuccessEvent,
} from '@atlaspack/types';
import path from 'path';

import {ServerDataProvider} from './ServerDataProvider';

/**
 * An implementation of ServerDataProvider provides data from a direct `bundleGraph`
 * and `requestBundle` function.
 */
export class StaticServerDataProvider implements ServerDataProvider {
  private distDir: string;

  private bundleGraph: BundleGraph<PackagedBundle> | null = null;

  private requestBundleFn:
    | ((bundle: PackagedBundle) => Promise<BuildSuccessEvent>)
    | null = null;

  constructor(distDir: string) {
    this.distDir = distDir;
  }

  getHTMLBundleFilePaths(): string[] {
    return (
      this.bundleGraph
        ?.getBundles()
        .filter((b) => path.posix.extname(b.name) === '.html')
        .map((b) => path.relative(this.distDir, b.filePath)) ?? []
    );
  }

  async requestBundle(
    requestedPath: string,
  ): Promise<'requested' | 'not-found'> {
    const bundle = this.bundleGraph?.getBundles().find((b) => {
      const relativePath = path.relative(this.distDir, b.filePath);
      return relativePath === requestedPath;
    });

    if (!bundle) {
      return 'not-found';
    }

    if (!this.requestBundleFn) {
      return 'not-found';
    }

    await this.requestBundleFn(bundle);

    return 'requested';
  }

  /**
   * Update the provider with the latest bundle graph and request function.
   *
   * This should be called after every successful build so that subsequent requests operate on fresh data.
   *
   * @param bundleGraph The most recent bundle graph representing the output of a build.
   * @param requestBundleFn Function that will be called to (re)build a specific bundle on demand.
   */
  update(
    bundleGraph: BundleGraph<PackagedBundle>,
    requestBundleFn: (bundle: PackagedBundle) => Promise<BuildSuccessEvent>,
  ) {
    this.bundleGraph = bundleGraph;
    this.requestBundleFn = requestBundleFn;
  }
}
