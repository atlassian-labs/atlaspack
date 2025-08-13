import type {
  BundleGraph,
  PackagedBundle,
  BuildSuccessEvent,
  PluginOptions,
} from '@atlaspack/types';
import path from 'path';
import type {Diagnostic} from '@atlaspack/diagnostic';
import {ansiHtml, FormattedCodeFrame, prettyDiagnostic} from '@atlaspack/utils';

import {ServerDataProvider} from './ServerDataProvider';

/**
 * An implementation of ServerDataProvider provides data from a direct `bundleGraph`
 * and `requestBundle` function.
 */
export class StaticServerDataProvider implements ServerDataProvider {
  private distDir: string;

  private status: 'initial-build' | 'build-pending' | 'idle' | 'build-failed' =
    'initial-build';

  private bundleGraph: BundleGraph<PackagedBundle> | null = null;

  private requestBundleFn:
    | ((bundle: PackagedBundle) => Promise<BuildSuccessEvent>)
    | null = null;

  private errors: Array<{
    message: string;
    stack: string | null | undefined;
    frames: Array<FormattedCodeFrame>;
    hints: Array<string>;
    documentation: string;
  }> | null = null;

  private nextBuildPromise: Promise<void> | null = null;
  private nextBuildResolve: (() => void) | null = null;
  private nextBuildReject: ((reason?: any) => void) | null = null;

  constructor(distDir: string) {
    this.distDir = distDir;
    this.nextBuildPromise = new Promise((resolve, reject) => {
      this.nextBuildResolve = resolve;
      this.nextBuildReject = reject;
    });
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
    if (this.status !== 'idle') {
      await this.nextBuildPromise;
    }

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

  onBuildStart() {
    if (this.status !== 'initial-build') {
      this.status = 'build-pending';
    }

    this.nextBuildPromise = new Promise((resolve, reject) => {
      this.nextBuildResolve = resolve;
      this.nextBuildReject = reject;
    });
  }

  async onBuildFailure(
    options: PluginOptions,
    diagnostics: Array<Diagnostic>,
  ): Promise<void> {
    this.status = 'build-failed';
    this.errors = await Promise.all(
      diagnostics.map(async (d) => {
        const ansiDiagnostic = await prettyDiagnostic(d, options);

        return {
          message: ansiHtml(ansiDiagnostic.message),
          stack: ansiDiagnostic.stack ? ansiHtml(ansiDiagnostic.stack) : null,
          frames: ansiDiagnostic.frames.map((f) => ({
            location: f.location,
            code: ansiHtml(f.code),
          })),
          hints: ansiDiagnostic.hints.map((hint) => ansiHtml(hint)),
          documentation: d.documentationURL ?? '',
        };
      }),
    );
    this.nextBuildReject?.(new Error('Build failed'));
  }

  onBuildSuccess(
    bundleGraph: BundleGraph<PackagedBundle>,
    requestBundleFn: (bundle: PackagedBundle) => Promise<BuildSuccessEvent>,
  ) {
    this.status = 'idle';
    this.errors = null;
    this.update(bundleGraph, requestBundleFn);
    this.nextBuildResolve?.();
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
