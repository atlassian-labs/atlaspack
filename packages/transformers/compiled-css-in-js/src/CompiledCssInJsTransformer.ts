import {getFeatureFlag} from '@atlaspack/feature-flags';
import {Transformer} from '@atlaspack/plugin';
import {
  applyCompiledCssInJsPlugin,
  type CompiledCssInJsTransformConfig,
} from '@atlaspack/rust/index';
import {join} from 'path';

const configFiles = ['.compiledcssrc', '.compiledcssrc.json'];

const PACKAGE_KEY = '@atlaspack/transformer-compiled-css-in-js';

export default new Transformer({
  async loadConfig({config, options}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return {};
    }

    const conf = await config.getConfigFrom<CompiledCssInJsTransformConfig>(
      join(options.projectRoot, 'index'),
      configFiles,
      {
        packageKey: PACKAGE_KEY,
      },
    );

    const contents: CompiledCssInJsTransformConfig = {};

    Object.assign(contents, conf?.contents);

    return contents;
  },
  async transform({asset, options, config, logger}) {
    if (!getFeatureFlag('compiledCssInJsTransformer')) {
      return [asset];
    }
    const code = await asset.getCode();
    if (code.includes('@compiled/react')) {
      const codeBuffer = Buffer.from(code);
      const compiledCode = await applyCompiledCssInJsPlugin(
        codeBuffer,
        options.projectRoot,
        asset.filePath,
        asset.isSource,
        config,
      );
      asset.setCode(compiledCode.toString());
    }
    return [asset];
  },
});
