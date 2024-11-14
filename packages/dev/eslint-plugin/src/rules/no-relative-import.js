// Forked from https://github.com/azz/eslint-plugin-monorepo/blob/master/src/rules/no-relative-import.js

/**
 * MIT License

  Copyright (c) 2017 Lucas Azzola

  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software and associated documentation files (the "Software"), to deal
  in the Software without restriction, including without limitation the rights
  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies of the Software, and to permit persons to whom the Software is
  furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in all
  copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
  SOFTWARE.
 */

const moduleVisitor = require('eslint-module-utils/moduleVisitor');

const getPackages = require('get-monorepo-packages');
const isInside = require('path-is-inside');
const minimatch = require('minimatch');
const {join, relative, parse} = require('path');
const resolve = require('eslint-module-utils/resolve');

const getPackageDir = (filePath, packages) => {
  const match = packages.find((pkg) =>
    minimatch(filePath, join(pkg.location, '**')),
  );

  if (match) {
    return match.location;
  }
};

module.exports = {
  meta: {
    docs: {
      description:
        'Disallow usage of relative imports instead of package names in monorepo',
    },
    fixable: 'code',
  },
  create(context) {
    const {
      options: [moduleUtilOptions],
    } = context;
    const sourceFsPath = context.getFilename();
    const packages = getPackages(process.cwd());

    return moduleVisitor.default((node) => {
      const resolvedPath = resolve.default(node.value, context);
      if (!resolvedPath) {
        return;
      }
      const packageDir = getPackageDir(sourceFsPath, packages);

      if (!packageDir || isInside(resolvedPath, packageDir)) {
        return;
      }

      const pkg = packages.find((pkg) => isInside(resolvedPath, pkg.location));
      if (!pkg) {
        return;
      }

      const subPackagePath = relative(pkg.location, resolvedPath);
      context.report({
        node,
        message: `Import for monorepo package '${pkg.package.name}' should be absolute.`,
        fix: (fixer) => {
          const {dir, name} = parse(
            `${pkg.package.name}${
              subPackagePath !== '.' ? `/${subPackagePath}` : ''
            }`,
          );

          return fixer.replaceText(
            node,
            `'${name !== '.' && name !== 'index' ? `${dir}/${name}` : dir}'`,
          );
        },
      });
    }, moduleUtilOptions);
  },
};
