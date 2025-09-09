const {Transformer} = require('@atlaspack/plugin');
module.exports = new Transformer({
  transform: async ({asset, options}) => {
    if (asset.env.unstableSingleFileOutput === false) {
      throw new Error('unstableSingleFileOutput should be true');
    }
    return [asset];
  },
});
