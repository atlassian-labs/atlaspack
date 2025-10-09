import {Transformer} from '@atlaspack/plugin';
import {applyTokensPlugin} from '@atlaspack/rust';
import path from 'path';

export default new Transformer({
  async transform({asset, options}) {
    const code = await asset.getCode();
    if (code.includes('@atlaskit/tokens')) {
      const codeBuffer = Buffer.from(code);
      // FIXME load this config from config
      const tokensPath = path.join(
        options.projectRoot,
        '../../../../../../afm/master/platform/packages/design-system/tokens/src/artifacts/token-data.json5',
        // './packages/design-system/tokens/src/artifacts/token-data.json',
      );
      const compiledCode = await applyTokensPlugin(
        codeBuffer,
        options.projectRoot,
        asset.filePath,
        asset.isSource,
        tokensPath,
      );
      asset.setBuffer(compiledCode as Buffer);
    }
    return [asset];
  },
}) as Transformer<unknown>;
