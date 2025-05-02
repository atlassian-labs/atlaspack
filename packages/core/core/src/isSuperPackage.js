// @flow strict-local
let fs = require('fs');
let {findAncestorFile} = require('@atlaspack/rust');

let dirname = /*#__ATLASPACK_IGNORE__*/ __dirname;

function isSuperPackage(): boolean {
  if (!dirname) {
    return false;
  }

  let packageJson = JSON.parse(
    // $FlowFixMe
    fs.readFileSync(findAncestorFile(['package.json'], dirname, '/'), 'utf8'),
  );

  return packageJson.name === '@atlaspack/super';
}

let result;

module.exports.isSuperPackage = (): boolean => {
  if (result == null) {
    result = isSuperPackage();
  }

  return result;
};
