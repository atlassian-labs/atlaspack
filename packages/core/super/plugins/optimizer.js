let path = require('path');
let {Optimizer} = require('@atlaspack/plugin');
let prettier = require('prettier');

module.exports = new Optimizer({
  async optimize({contents}) {
    let formattedContents = await prettier.format(contents, {parser: 'babel'});

    return {contents: formattedContents};
  },
});
