let path = require('path');
let {Resolver} = require('@atlaspack/plugin');

module.exports = new Resolver({
  resolve({dependency, specifier}) {
    if (
      specifier === './internal-plugins' &&
      dependency.sourcePath.includes(
        'packages/core/core/src/loadAtlaspackPlugin.js',
      )
    ) {
      return {
        filePath: path.join(__dirname, '../entries/internal-plugins.js'),
      };
    }
  },
});
