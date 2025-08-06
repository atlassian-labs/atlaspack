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
  distDir: string;
  bundleGraph: BundleGraph<PackagedBundle> | null = null;
  requestBundleFn:
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
    const bundle = this.bundleGraph
      ?.getBundles()
      .find((b) => path.relative(this.distDir, b.filePath) === requestedPath);

    if (!bundle) {
      return 'not-found';
    }

    if (!this.requestBundleFn) {
      return 'not-found';
    }

    await this.requestBundleFn(bundle);

    return 'requested';
  }

  update(
    bundleGraph: BundleGraph<PackagedBundle>,
    requestBundleFn: (bundle: PackagedBundle) => Promise<BuildSuccessEvent>,
  ) {
    this.bundleGraph = bundleGraph;
    this.requestBundleFn = requestBundleFn;
  }
}
