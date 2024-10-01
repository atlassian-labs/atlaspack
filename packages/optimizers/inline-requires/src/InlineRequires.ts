import {Optimizer} from '@atlaspack/plugin';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {runInlineRequiresOptimizer} from '@atlaspack/rust';
import {parse, print} from '@swc/core';
import {RequireInliningVisitor} from './RequireInliningVisitor';
import nullthrows from 'nullthrows';
import SourceMap from '@parcel/source-map';

// @ts-expect-error - TS7034 - Variable 'assetPublicIdsWithSideEffects' implicitly has type 'any' in some locations where its type cannot be determined.
let assetPublicIdsWithSideEffects = null;

type BundleConfig = {
  assetPublicIdsWithSideEffects: Set<string>;
};

// @ts-expect-error - TS2558 - Expected 0 type arguments, but got 2.
module.exports = new Optimizer<never, BundleConfig>({
  loadBundleConfig({bundle, bundleGraph, tracer}): BundleConfig {
    // @ts-expect-error - TS7005 - Variable 'assetPublicIdsWithSideEffects' implicitly has an 'any' type.
    if (assetPublicIdsWithSideEffects !== null) {
      // @ts-expect-error - TS7005 - Variable 'assetPublicIdsWithSideEffects' implicitly has an 'any' type.
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
        // @ts-expect-error - TS7005 - Variable 'assetPublicIdsWithSideEffects' implicitly has an 'any' type.
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
    tracer,
    logger,
    bundleConfig,
    options,
  }) {
    if (!bundle.env.shouldOptimize) {
      return {contents, map: originalMap};
    }

    try {
      if (getFeatureFlag('fastOptimizeInlineRequires')) {
        let sourceMap = null;
        const result = runInlineRequiresOptimizer({
          code: contents.toString(),
          sourceMaps: !!bundle.env.sourceMap,
          ignoreModuleIds: Array.from(
            // @ts-expect-error - TS2571 - Object is of type 'unknown'.
            bundleConfig.assetPublicIdsWithSideEffects,
          ),
        });
        const sourceMapResult = result.sourceMap;
        if (sourceMapResult != null) {
          sourceMap = new SourceMap(options.projectRoot);
          sourceMap.addVLQMap(JSON.parse(sourceMapResult));
          if (originalMap) {
            // @ts-expect-error - TS2345 - Argument of type 'SourceMap' is not assignable to parameter of type 'Buffer'.
            sourceMap.extends(originalMap);
          }
        }
        return {contents: result.code, map: originalMap};
      }

      let measurement = tracer.createMeasurement(
        '@atlaspack/optimizer-inline-requires',
        'parse',
        bundle.name,
      );
      const ast = await parse(contents.toString());
      measurement && measurement.end();

      const visitor = new RequireInliningVisitor({
        bundle,
        logger,
        assetPublicIdsWithSideEffects:
          // @ts-expect-error - TS2571 - Object is of type 'unknown'.
          bundleConfig.assetPublicIdsWithSideEffects,
      });

      measurement = tracer.createMeasurement(
        '@atlaspack/optimizer-inline-requires',
        'visit',
        bundle.name,
      );
      visitor.visitProgram(ast);
      measurement && measurement.end();

      if (visitor.dirty) {
        const measurement = tracer.createMeasurement(
          '@atlaspack/optimizer-inline-requires',
          'print',
          bundle.name,
        );
        const result = await print(ast, {sourceMaps: !!bundle.env.sourceMap});
        measurement && measurement.end();

        let sourceMap = null;
        let resultMap = result.map;
        let contents: string = nullthrows(result.code);

        if (resultMap != null) {
          sourceMap = new SourceMap(options.projectRoot);
          sourceMap.addVLQMap(JSON.parse(resultMap));
          if (originalMap) {
            // @ts-expect-error - TS2345 - Argument of type 'SourceMap' is not assignable to parameter of type 'Buffer'.
            sourceMap.extends(originalMap);
          }
        }

        return {contents, map: sourceMap};
      }
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
