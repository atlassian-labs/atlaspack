// @flow strict-local
import {relative, join} from 'path';
import {Reporter} from '@atlaspack/plugin';
import type {
  Async,
  PluginLogger,
  PluginOptions,
  PluginTracer,
  ReporterEvent,
} from '@atlaspack/types-internal';
import {getConfig} from './Config';

async function report({
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
    for (const [bundle, conditions] of bundles.entries()) {
      const bundleInfo = {};
      for (const [key, cond] of conditions) {
        bundleInfo[key] = {
          // Reverse bundles so we load children bundles first
          ifTrueBundles: mapBundles(cond.ifTrueBundles).reverse(),
          ifFalseBundles: mapBundles(cond.ifFalseBundles).reverse(),
        };
      }

      manifest[bundle.target.name] ??= {};
      manifest[bundle.target.name][
        relative(bundle.target.distDir, bundle.filePath)
      ] = bundleInfo;
    }

    const targets = new Set(
      event.bundleGraph.getBundles().map((bundle) => bundle.target),
    );

    const {filename} = await getConfig(options);

    for (const target of targets) {
      const conditionalManifestFilename = join(target.distDir, filename);

      const conditionalManifest = JSON.stringify(
        manifest[target.name],
        null,
        2,
      );

      await options.outputFS.writeFile(
        conditionalManifestFilename,
        conditionalManifest,
        {mode: 0o666},
      );

      logger.info({
        message: 'Wrote conditional manifest to ' + conditionalManifestFilename,
      });
    }
  }
}

export default (new Reporter({report}): Reporter);
