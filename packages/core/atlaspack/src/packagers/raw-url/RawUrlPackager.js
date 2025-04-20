// @flow strict-local

import assert from 'assert';
import {Packager} from '@atlaspack/plugin';
import {replaceURLReferences} from '@atlaspack/utils';

export default (new Packager({
  async package({bundle, bundleGraph}) {
    let assets = [];
    bundle.traverseAssets((asset) => {
      assets.push(asset);
    });

    assert.equal(assets.length, 1, 'Raw bundles must only contain one asset');
    let {contents} = replaceURLReferences({
      bundle,
      bundleGraph,
      contents: await assets[0].getCode(),
      relative: false,
      getReplacement: (s) => s,
    });
    return {contents};
  },
}): Packager);
