import {Transformer} from '@atlaspack/plugin';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'js-yaml'. '/home/ubuntu/parcel/packages/transformers/yaml/node_modules/js-yaml/index.js' implicitly has an 'any' type.
import yaml from 'js-yaml';

export default new Transformer({
  async transform({asset}) {
    asset.type = 'js';
    asset.setCode(
      `module.exports = ${JSON.stringify(
        yaml.load(await asset.getCode()),
        null,
        2,
      )};`,
    );
    return [asset];
  },
}) as Transformer;
