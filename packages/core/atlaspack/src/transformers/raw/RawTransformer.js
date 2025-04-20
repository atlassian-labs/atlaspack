// @flow strict-local

import {Transformer} from '../../plugin/index.js';

export default (new Transformer({
  transform({asset}) {
    asset.bundleBehavior = 'isolated';
    return [asset];
  },
}): Transformer);
