// @flow

import type {
  BuildSuccessEvent,
  Dependency,
  PluginOptions,
  BundleGraph,
  PackagedBundle,
  Asset,
} from '@atlaspack/types';
import type {Diagnostic} from '@atlaspack/diagnostic';
import type {AnsiDiagnosticResult} from '@atlaspack/utils';

import invariant from 'assert';
import {ansiHtml, prettyDiagnostic, PromiseQueue} from '@atlaspack/utils';

const HMR_ENDPOINT = '/__atlaspack_hmr/';

type HMRAsset = {|
  id: string,
  url: string,
  type: string,
  output: string,
  envHash: string,
  depsByBundle: {[string]: {[string]: string, ...}, ...},
|};

export type HMRMessage =
  | {|
      type: 'update',
      assets: Array<HMRAsset>,
    |}
  | {|
      type: 'error',
      diagnostics: {|
        ansi: Array<AnsiDiagnosticResult>,
        html: Array<$Rest<AnsiDiagnosticResult, {|codeframe: string|}>>,
      |},
    |};

const FS_CONCURRENCY = 64;

export default class HMRServer {
  unresolvedError: HMRMessage | null = null;
  broadcast: HMRMessage => void;

  constructor(broadcast: HMRMessage => void) {
    this.broadcast = broadcast;
  }

  async emitError(options: PluginOptions, diagnostics: Array<Diagnostic>) {
    let renderedDiagnostics = await Promise.all(
      diagnostics.map(d => prettyDiagnostic(d, options)),
    );

    // store the most recent error so we can notify new connections
    // and so we can broadcast when the error is resolved
    this.unresolvedError = {
      type: 'error',
      diagnostics: {
        ansi: renderedDiagnostics,
        html: renderedDiagnostics.map((d, i) => {
          return {
            message: ansiHtml(d.message),
            stack: ansiHtml(d.stack),
            frames: d.frames.map(f => ({
              location: f.location,
              code: ansiHtml(f.code),
            })),
            hints: d.hints.map(hint => ansiHtml(hint)),
            documentation: diagnostics[i].documentationURL ?? '',
          };
        }),
      },
    };

    this.broadcast(this.unresolvedError);
  }

  async emitUpdate(event: BuildSuccessEvent) {
    this.unresolvedError = null;

    let changedAssets = new Set(event.changedAssets.values());
    if (changedAssets.size === 0) return;

    let queue = new PromiseQueue({maxConcurrent: FS_CONCURRENCY});
    for (let asset of changedAssets) {
      if (asset.type !== 'js' && asset.type !== 'css') {
        // If all of the incoming dependencies of the asset actually resolve to a JS asset
        // rather than the original, we can mark the runtimes as changed instead. URL runtimes
        // have a cache busting query param added with HMR enabled which will trigger a reload.
        let runtimes = new Set();
        let incomingDeps = event.bundleGraph.getIncomingDependencies(asset);
        let isOnlyReferencedByRuntimes = incomingDeps.every(dep => {
          let resolved = event.bundleGraph.getResolvedAsset(dep);
          let isRuntime = resolved?.type === 'js' && resolved !== asset;
          if (resolved && isRuntime) {
            runtimes.add(resolved);
          }
          return isRuntime;
        });

        if (isOnlyReferencedByRuntimes) {
          for (let runtime of runtimes) {
            changedAssets.add(runtime);
          }

          continue;
        }
      }

      queue.add(async () => {
        let dependencies = event.bundleGraph.getDependencies(asset);
        let depsByBundle = {};
        for (let bundle of event.bundleGraph.getBundlesWithAsset(asset)) {
          let deps = {};
          for (let dep of dependencies) {
            let resolved = event.bundleGraph.getResolvedAsset(dep, bundle);
            if (resolved) {
              deps[getSpecifier(dep)] =
                event.bundleGraph.getAssetPublicId(resolved);
            }
          }
          depsByBundle[bundle.id] = deps;
        }

        return {
          id: event.bundleGraph.getAssetPublicId(asset),
          url: getSourceURL(event.bundleGraph, asset),
          type: asset.type,
          // No need to send the contents of non-JS assets to the client.
          output:
            asset.type === 'js'
              ? await getHotAssetContents(event.bundleGraph, asset)
              : '',
          envHash: asset.env.id,
          depsByBundle,
        };
      });
    }

    let assets = await queue.run();
    this.broadcast({
      type: 'update',
      assets: assets,
    });
  }
}

function getSpecifier(dep: Dependency): string {
  if (typeof dep.meta.placeholder === 'string') {
    return dep.meta.placeholder;
  }

  return dep.specifier;
}

export async function getHotAssetContents(
  bundleGraph: BundleGraph<PackagedBundle>,
  asset: Asset,
): Promise<string> {
  let output = await asset.getCode();
  if (asset.type === 'js') {
    let publicId = bundleGraph.getAssetPublicId(asset);
    output = `atlaspackHotUpdate['${publicId}'] = function (require, module, exports) {${output}}`;
  }

  let sourcemap = await asset.getMap();
  if (sourcemap) {
    let sourcemapStringified = await sourcemap.stringify({
      format: 'inline',
      sourceRoot: '/__atlaspack_source_root/',
      // $FlowFixMe
      fs: asset.fs,
    });

    invariant(typeof sourcemapStringified === 'string');
    output += `\n//# sourceMappingURL=${sourcemapStringified}`;
    output += `\n//# sourceURL=${getSourceURL(bundleGraph, asset)}\n`;
  }

  return output;
}

function getSourceURL(bundleGraph, asset) {
  return HMR_ENDPOINT + asset.id;
}
