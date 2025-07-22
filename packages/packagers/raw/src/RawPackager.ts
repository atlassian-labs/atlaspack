import assert from 'assert';
import {Packager} from '@atlaspack/plugin';

export default new Packager({
  package({bundle}) {
    // @ts-expect-error TS2552
    let assets: Array<Asset> = [];
    bundle.traverseAssets((asset) => {
      assets.push(asset);
    });

    assert.equal(assets.length, 1, 'Raw bundles must only contain one asset');
    return {contents: assets[0].getStream()};
  },
}) as Packager<unknown, unknown>;
