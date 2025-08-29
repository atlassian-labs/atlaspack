// This is a real file so that it works with V3.
// It's a simple transformer that replaces MY_ENV with the target value.

const {Transformer} = require('@atlaspack/plugin');

module.exports = new Transformer({
  transform: async ({asset, options}) => {
    const customEnv = asset.env.customEnv;
    const code = await asset.getCode();
    const newCode = code.replace(/MY_ENV/g, customEnv.MY_ENV || 'MY_ENV');
    asset.setCode(newCode);
    return [asset];
  }
});