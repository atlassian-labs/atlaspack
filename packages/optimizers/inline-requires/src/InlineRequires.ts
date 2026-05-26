import {Optimizer} from '@atlaspack/plugin';
import {runInlineRequiresOptimizerAsync} from '@atlaspack/rust';
import nullthrows from 'nullthrows';
import SourceMap from '@atlaspack/source-map';

// @ts-expect-error TS7034
let assetPublicIdsWithSideEffects = null;

type BundleConfig = {
  assetPublicIdsWithSideEffects: Set<string>;
};

module.exports = new Optimizer<never, BundleConfig>({
  loadBundleConfig({bundle, bundleGraph, tracer}): BundleConfig {
    // @ts-expect-error TS7005
    if (assetPublicIdsWithSideEffects !== null) {
      // @ts-expect-error TS7005
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
        // @ts-expect-error TS7005
        let sideEffectsMap = nullthrows(assetPublicIdsWithSideEffects);
        sideEffectsMap.add(publicId);
      }
    });

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
      // Hand the upstream map straight to the Rust optimizer so it can
      // compose the result map via swc's native source-map chaining,
      // rather than emitting an `<anon>`-rooted map and stitching it on
      // afterwards with `SourceMap.extends()`. That JS-side composition
      // uses `find_closest_mapping`, which mis-attributes tokens near
      // asset boundaries after downstream minification.
      const inputSourceMap =
        originalMap && bundle.env.sourceMap
          ? // swc requires version: 3; atlaspack's `toVLQ()` omits it.
            JSON.stringify({version: 3, ...originalMap.toVLQ()})
          : undefined;
      const result = await runInlineRequiresOptimizerAsync({
        code: contents.toString(),
        sourceMaps: !!bundle.env.sourceMap,
        ignoreModuleIds: Array.from(bundleConfig.assetPublicIdsWithSideEffects),
        inputSourceMap,
      });

      // @ts-expect-error TS2339
      const sourceMapResult = result.sourceMap;
      if (sourceMapResult != null) {
        sourceMap = new SourceMap(options.projectRoot);
        sourceMap.addVLQMap(JSON.parse(sourceMapResult));
        // No extends() needed: the Rust side already composed against
        // `inputSourceMap`, so the result map already references the
        // original sources directly.
      }
      // @ts-expect-error TS2339
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
