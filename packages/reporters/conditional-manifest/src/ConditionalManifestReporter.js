// @flow strict-local
import {relative, join} from 'path';
import {Reporter} from '@atlaspack/plugin';

export default (new Reporter({
  async report({event, options, logger}) {
    if (event.type === 'buildSuccess') {
      const bundles = event.bundleGraph.getConditionalBundleMapping();

      // Replace bundles with file paths
      const mapBundles = (bundles) =>
        bundles.map((bundle) =>
          relative(bundle.target.distDir, bundle.filePath),
        );

      const manifest = {};
      for (const [bundle, conditions] of bundles.entries()) {
        const bundleInfo = {};
        for (const [key, cond] of conditions) {
          bundleInfo[key] = {
            ifTrueBundles: mapBundles(cond.ifTrueBundles).reverse(),
            ifFalseBundles: mapBundles(cond.ifFalseBundles).reverse(),
          };
        }

        manifest[bundle.target.name] ??= {};
        manifest[bundle.target.name][
          relative(bundle.target.distDir, bundle.filePath)
        ] = bundleInfo;
      }

      // Error if there are multiple targets in the build
      const targets = new Set(
        event.bundleGraph.getBundles().map((bundle) => bundle.target),
      );

      for (const target of targets) {
        const conditionalManifestFilename = join(
          target.distDir,
          'conditional-manifest.json',
        );

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
          message:
            'Wrote conditional manifest to ' + conditionalManifestFilename,
        });
      }
    }
  },
}): Reporter);
