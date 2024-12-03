// @flow strict-local
import {Optimizer} from '@atlaspack/plugin';
import {runInlineRequiresOptimizer} from '@atlaspack/rust';
import nullthrows from 'nullthrows';
import SourceMap from '@parcel/source-map';

let assetPublicIdsWithSideEffects = null;

type BundleConfig = {|
  assetPublicIdsWithSideEffects: Set<string>,
|};

// $FlowFixMe not sure how to anotate the export here to make it work...
module.exports = new Optimizer<empty, BundleConfig>({
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

    measurement && measurement.end();

    return {assetPublicIdsWithSideEffects};
  },

  optimize({
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
      const result = runInlineRequiresOptimizer({
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
      return {contents: result.code, map: originalMap};
    } catch (err) {
      logger.warn({
        origin: 'atlaspack-optimizer-experimental-inline-requires',
        message: `Unable to optimise requires for ${bundle.name}: ${err.message}`,
        stack: err.stack,
      });
    }
    return {contents, map: originalMap};
  },
});
