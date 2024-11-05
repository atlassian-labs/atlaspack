/* eslint-disable import/no-extraneous-dependencies */
const {Transformer} = require('@atlaspack/plugin');

module.exports = new Transformer({
  loadConfig() {
    return null;
  },
  transform({asset}) {
    return [asset];
  },
});
