let fs = require('fs');
let {findAncestorFile} = require('@atlaspack/rust');

let dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;

function isSuperPackage() {
  if (!dirname) {
    return false;
  }

  let packageJson = JSON.parse(
    fs.readFileSync(findAncestorFile(['package.json'], dirname, '/')),
  );

  return packageJson.name === '@atlaspack/super';
}

let result;

module.exports.isSuperPackage = () => {
  if (result == null) {
    result = isSuperPackage();
  }

  return result;
};
