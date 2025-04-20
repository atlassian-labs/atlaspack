// @flow strict-local

import {Transformer} from '../../plugin/index.js';

export default (new Transformer({
  transform({asset}) {
    asset.bundleBehavior = 'inline';
    asset.meta.inlineType = 'string';
    return [asset];
  },
}): Transformer);
