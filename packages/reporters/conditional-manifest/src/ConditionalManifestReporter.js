// @flow strict-local
import {relative, join, dirname} from 'path';
import crypto from 'crypto';
import {Reporter} from '@atlaspack/plugin';
import type {
  Async,
  PluginLogger,
  PluginOptions,
  PluginTracer,
  ReporterEvent,
  FileSystem,
  FilePath,
} from '@atlaspack/types-internal';
import {getConfig} from './Config';

export const manifestHashes: Map<FilePath, string> = new Map();

export const updateManifest = async (
  outputFS: FileSystem,
  logger: PluginLogger,
  conditionalManifestFilename: FilePath,
  conditionalManifest: string,
) => {
  const hash = crypto
    .createHash('sha1')
    .update(conditionalManifest)
    .digest('hex');

  if (manifestHashes.get(conditionalManifestFilename) !== hash) {
    manifestHashes.set(conditionalManifestFilename, hash);

    await outputFS.mkdirp(dirname(conditionalManifestFilename));
    await outputFS.writeFile(conditionalManifestFilename, conditionalManifest, {
      mode: 0o666,
    });

    logger.info({
      message: `Wrote conditional manifest to ${conditionalManifestFilename}`,
    });
  }
};

export async function report({
  event,
  options,
  logger,
}: {|
  event: ReporterEvent,
  options: PluginOptions,
  logger: PluginLogger,
  tracer: PluginTracer,
|}): Async<void> {
  if (event.type === 'buildSuccess') {
    const bundles = event.bundleGraph.getConditionalBundleMapping();

    // Replace bundles with file paths
    const mapBundles = (bundles) =>
      bundles.map((bundle) => relative(bundle.target.distDir, bundle.filePath));

    const manifest = {};
    for (const conditions of bundles.values()) {
      for (const [key, cond] of conditions) {
        const bundle = cond.bundle;
        const relativeBundlePath = relative(
          bundle.target.distDir,
          bundle.filePath,
        );

        const bundleInfo =
          manifest[bundle.target.name]?.[relativeBundlePath] ?? {};

        bundleInfo[key] = {
          ifTrueBundles: mapBundles(cond.ifTrueBundles)
            .concat(bundleInfo[key]?.ifTrueBundles ?? [])
            .sort(),
          ifFalseBundles: mapBundles(cond.ifFalseBundles)
            .concat(bundleInfo[key]?.ifFalseBundles ?? [])
            .sort(),
        };

        manifest[bundle.target.name] ??= {};
        manifest[bundle.target.name][relativeBundlePath] = bundleInfo;
      }
    }

    const targets = new Set(
      event.bundleGraph.getBundles().map((bundle) => bundle.target),
    );

    const {filename} = await getConfig(options);

    for (const target of targets) {
      const conditionalManifestFilename = join(target.distDir, filename);
      const conditionalManifest = JSON.stringify(
        // If there's no content, send an empty manifest so we can still map from it safely
        manifest[target.name] ?? {},
        null,
        2,
      );

      await updateManifest(
        options.outputFS,
        logger,
        conditionalManifestFilename,
        conditionalManifest,
      );
    }
  }
}

export default (new Reporter({report}): Reporter);
