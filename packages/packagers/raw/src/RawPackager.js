// @flow strict-local

import assert from 'assert';
import {Packager} from '@atlaspack/plugin';

export default (new Packager({
  package({bundle}) {
    let assets = [];
    bundle.traverseAssets((asset) => {
      assets.push(asset);
    });

    assert.equal(assets.length, 1, 'Raw bundles must only contain one asset');
    return {contents: assets[0].getStream()};
  },
}): Packager<mixed, mixed>);
