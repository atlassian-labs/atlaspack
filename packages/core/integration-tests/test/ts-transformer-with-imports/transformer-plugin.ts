import {Transformer} from '@atlaspack/plugin';
import {replacementValue} from './helpers';

export default new Transformer({
  async transform({asset}) {
    const code = await asset.getCode();
    if (code.includes('MARKER_VALUE')) {
      asset.setCode('module.exports = "' + replacementValue + '";');
    }
    return [asset];
  },
});
