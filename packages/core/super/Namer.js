let {Namer} = require('@atlaspack/plugin');

module.exports = new Namer({
  name({bundle, bundleGraph}) {
    let entryAsset = bundle.getMainEntry();
    if (entryAsset?.filePath.includes('/node_modules/')) {
      return `vendor.${bundle.hashReference}.${bundle.type}`;
    }
  },
});
