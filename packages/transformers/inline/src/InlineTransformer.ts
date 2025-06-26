import {Transformer} from '@atlaspack/plugin';

export default new Transformer({
  transform({asset}) {
    asset.bundleBehavior = 'inline';
    return [asset];
  },
}) as Transformer;
