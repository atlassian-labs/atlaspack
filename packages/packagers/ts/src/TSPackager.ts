import assert from 'assert';
import {Packager} from '@atlaspack/plugin';

export default new Packager({
  async package({bundle, getSourceMapReference}) {
    // @ts-expect-error TS2552
    let assets: Array<Asset> = [];
    bundle.traverseAssets((asset) => {
      assets.push(asset);
    });

    assert.equal(assets.length, 1, 'TS bundles must only contain one asset');
    let code = await assets[0].getCode();
    let map = await assets[0].getMap();
    if (map) {
      let sourcemapReference = await getSourceMapReference(map);
      if (sourcemapReference != null) {
        code += '\n//# sourceMappingURL=' + sourcemapReference + '\n';
      }
    }

    return {contents: code, map};
  },
}) as Packager<unknown, unknown>;
