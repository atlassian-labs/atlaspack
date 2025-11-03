import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {
  applyCompiledCssInJsPlugin,
  CompiledCssInJsPluginResult,
  type CompiledCssInJsConfig,
} from '@atlaspack/rust/index';
import {join} from 'path';
import SourceMap from '@parcel/source-map';

const configFiles = ['.compiledcssrc', '.compiledcssrc.json'];

const PACKAGE_KEY = '@atlaspack/transformer-compiled-css-in-js';

export default new Transformer({
  async loadConfig({config, options}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return {};
    }

    const conf = await config.getConfigFrom<CompiledCssInJsConfig>(
      join(options.projectRoot, 'index'),
      configFiles,
      {
        packageKey: PACKAGE_KEY,
      },
    );

    const contents: CompiledCssInJsConfig = {};

    Object.assign(contents, conf?.contents);

    return contents;
  },
  async transform({asset, options, config, logger}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return [asset];
    }

    const mapPromise = asset.getMap();
    const code = await asset.getCode();
    if (
      config.importSources?.every(
        (source) =>
          !code.includes(source) || code.includes(source + '/runtime'),
      )
    ) {
      return [asset];
    }

    if (
      code.includes('styled.') ||
      code.includes('styled(') ||
      code.includes('cssMap')
    ) {
      return [asset];
    }

    const codeBuffer = Buffer.from(code);

    const result = (await applyCompiledCssInJsPlugin(codeBuffer, {
      filename: asset.filePath,
      projectRoot: options.projectRoot,
      isSource: asset.isSource,
      sourceMaps: !!asset.env.sourceMap,
      config,
    })) as CompiledCssInJsPluginResult;

    // Handle sourcemap merging if sourcemap is generated
    if (result.map != null) {
      let map = new SourceMap(options.projectRoot);
      map.addVLQMap(JSON.parse(result.map));
      const originalMap = await mapPromise;
      if (originalMap) {
        // @ts-expect-error TS2345 - the types are wrong, `extends` accepts a `SourceMap` or a `Buffer`
        map.extends(originalMap);
      }
      asset.setMap(map);
    }

    // Rather then setting this as a buffer we set it as a string, since most of the following
    // plugins will call `getCode`, this avoids repeatedly converting the buffer to a string.
    asset.setCode(result.code);

    // Add styleRules to the asset
    asset.meta.styleRules = result.styleRules;

    return [asset];
  },
});
