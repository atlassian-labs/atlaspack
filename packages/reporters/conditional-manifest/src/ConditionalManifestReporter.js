import {relative, join} from 'node:path';
import {Reporter} from '@atlaspack/plugin';

export default new Reporter({
  async report({event, options, logger}) {
    if (event.type === 'buildSuccess') {
      const conditions =
        event.bundleGraph.unstable_getConditionalBundleMapping();
      // console.log(JSON.stringify(conditions, null, 2));
      // Replace bundles with file paths..
      const mapBundles = bundles =>
        bundles.map(bundle => relative(bundle.target.distDir, bundle.filePath));
      for (const [, cond] of Object.entries(conditions)) {
        cond.bundlesWithCondition = mapBundles(cond.bundlesWithCondition);
        cond.ifTrueBundles = mapBundles(cond.ifTrueBundles).reverse();
        cond.ifFalseBundles = mapBundles(cond.ifFalseBundles).reverse();
      }
      const conditionalManifest = JSON.stringify(conditions, null, 2);

      const targets = new Set(
        event.bundleGraph.getBundles().map(bundle => bundle.target),
      );
      if (targets.size > 1) {
        throw new Error(
          'Conditional bundling does not support multiple targets',
        );
      }
      const target = targets.values().next().value;
      const conditionalManifestFilename = join(
        target.distDir,
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
});
