// @flow strict-local
import {relative, join} from 'path';
import {Reporter} from '@atlaspack/plugin';
import nullthrows from 'nullthrows';

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

        manifest[relative(bundle.target.distDir, bundle.filePath)] = bundleInfo;
      }

      const conditionalManifest = JSON.stringify(manifest, null, 2);

      // Error if there are multiple targets in the build
      const targets = new Set(
        event.bundleGraph.getBundles().map((bundle) => bundle.target),
      );
      if (targets.size > 1) {
        throw new Error(
          'Conditional bundling does not support multiple targets',
        );
      }

      const target = targets.values().next().value;
      const conditionalManifestFilename = join(
        nullthrows(target?.distDir, 'distDir not found in target'),
        'conditional-manifest.json',
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
  },
}): Reporter);
