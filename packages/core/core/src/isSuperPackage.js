let fs = require('fs');
let path = require('path');
let {findAncestorFile} = require('@atlaspack/rust');
let dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;

module.exports.isSuperPackage = () => {
  if (!dirname) {
    return false;
  }

  let packageJson = JSON.parse(
    fs.readFileSync(findAncestorFile(['package.json'], dirname, '/')),
  );

  return packageJson.name === '@atlaspack/super';
};
