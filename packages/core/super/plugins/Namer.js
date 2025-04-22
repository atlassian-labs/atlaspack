let path = require('path');
let {Namer} = require('@atlaspack/plugin');

module.exports = new Namer({
  name({bundle, options}) {
    let entryAsset = bundle.getMainEntry();

    if (bundle.needsStableName) {
      // This is an entry so just use it's base file name
      return path.basename(entryAsset.filePath);
    }

    if (entryAsset?.filePath.includes('/node_modules/')) {
      // This is the vendor bundle
      return `vendor.${bundle.hashReference}.${bundle.type}`;
    }

    let projectRelativePath = path.relative(
      path.join(options.projectRoot, '../..'),
      entryAsset.filePath,
    );

    if (projectRelativePath.startsWith('..')) {
      throw new Error(
        `Invalid relative path: "${projectRelativePath}" from "${options.projectRoot}"`,
      );
    }

    if (projectRelativePath.endsWith('.json')) {
      // JSON files are actually compiled to JS files
      projectRelativePath = projectRelativePath + '.js';
    }

    // This must be another Atlaspack import so just name it to match it's
    // repo file path
    return projectRelativePath.replaceAll('/src/', '/');
  },
});
