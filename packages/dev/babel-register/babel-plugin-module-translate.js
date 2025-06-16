const fs = require('fs');
const resolve = require('resolve');
const path = require('path');

function resolveSource(specifier, from, isSuperPackageMode) {
  if (isSuperPackageMode) {
    let superPath = path.join(__dirname, '../../core/super/lib');
    let thing = specifier.substring('@atlaspack/'.length);

    let result = path.join(superPath, thing + '.js');

    if (!fs.existsSync(result)) {
      return specifier;
    }

    return result;
  }

  return resolve.sync(specifier, {
    basedir: path.dirname(from),
    packageFilter(pkg) {
      if (pkg.name.startsWith('@atlaspack/')) {
        if (pkg.source) {
          pkg.main = pkg.source;
        }
      }
      return pkg;
    },
  });
}

let sourceFieldCache = new Map();
function getSourceField(specifier, from, isSuperPackageMode) {
  let key = `${specifier}:${from}:${isSuperPackageMode}`;
  if (sourceFieldCache.has(key)) {
    return sourceFieldCache.get(key);
  }

  let result = resolveSource(specifier, from, isSuperPackageMode);
  sourceFieldCache.set(key, result);
  return result;
}

module.exports = ({types: t}) => ({
  name: 'module-translate',
  visitor: {
    ImportDeclaration({node}, state) {
      let source = node.source;
      if (t.isStringLiteral(source) && source.value.startsWith('@atlaspack/')) {
        source.value = getSourceField(
          source.value,
          state.file.opts.filename || process.cwd(),
          state.opts.superPackage,
        );
      }
    },
    CallExpression(path, state) {
      let {node} = path;
      if (
        t.isIdentifier(node.callee, {name: 'require'}) &&
        !path.scope.hasBinding(node.callee.value) &&
        node.arguments.length === 1 &&
        t.isStringLiteral(node.arguments[0]) &&
        node.arguments[0].value.startsWith('@atlaspack/')
      ) {
        try {
          node.arguments[0].value = getSourceField(
            node.arguments[0].value,
            state.file.opts.filename || process.cwd(),
            state.opts.superPackage,
          );
        } catch (e) {
          let exprStmtParent = path
            .getAncestry()
            .find((v) => v.isExpressionStatement());
          if (exprStmtParent) {
            exprStmtParent.replaceWith(
              t.throwStatement(t.stringLiteral(e.message)),
            );
          }
        }
      }
    },
  },
});

module.exports.resolveSource = resolveSource;
