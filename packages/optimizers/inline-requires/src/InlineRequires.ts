import {Optimizer} from '@atlaspack/plugin';
import {runInlineRequiresOptimizerAsync} from '@atlaspack/rust';
import nullthrows from 'nullthrows';
import SourceMap from '@parcel/source-map';

let assetPublicIdsWithSideEffects = null;

type BundleConfig = {
  assetPublicIdsWithSideEffects: Set<string>;
};

module.exports = new Optimizer<never, BundleConfig>({
  loadBundleConfig({bundle, bundleGraph, tracer}): BundleConfig {
    if (assetPublicIdsWithSideEffects !== null) {
      return {assetPublicIdsWithSideEffects};
    }

    assetPublicIdsWithSideEffects = new Set<string>();

    if (!bundle.env.shouldOptimize) {
      return {assetPublicIdsWithSideEffects};
    }

    const measurement = tracer.createMeasurement(
      '@atlaspack/optimizer-inline-requires',
      'generatePublicIdToAssetSideEffects',
      bundle.name,
    );

    bundleGraph.traverse((node) => {
      if (node.type === 'asset' && node.value.sideEffects) {
        const publicId = bundleGraph.getAssetPublicId(node.value);
        let sideEffectsMap = nullthrows(assetPublicIdsWithSideEffects);
        sideEffectsMap.add(publicId);
      }
    });

    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    measurement && measurement.end();

    return {assetPublicIdsWithSideEffects};
  },

  async optimize({
    bundle,
    contents,
    map: originalMap,
    logger,
    bundleConfig,
    options,
  }) {
    if (!bundle.env.shouldOptimize) {
      return {contents, map: originalMap};
    }

    try {
      let sourceMap = null;
      const result = await runInlineRequiresOptimizerAsync({
        code: contents.toString(),
        sourceMaps: !!bundle.env.sourceMap,
        ignoreModuleIds: Array.from(bundleConfig.assetPublicIdsWithSideEffects),
      });

      const sourceMapResult = result.sourceMap;
      if (sourceMapResult != null) {
        sourceMap = new SourceMap(options.projectRoot);
        sourceMap.addVLQMap(JSON.parse(sourceMapResult));
        if (originalMap) {
          sourceMap.extends(originalMap);
        }
      }
      return {contents: result.code, map: sourceMap};
    } catch (err: any) {
      logger.warn({
        origin: 'atlaspack-optimizer-experimental-inline-requires',
        message: `Unable to optimise requires for ${bundle.name}: ${err.message}`,
        stack: err.stack,
      });
    }
    return {contents, map: originalMap};
  },
});
