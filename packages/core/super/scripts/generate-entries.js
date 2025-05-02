/* eslint-disable import/no-extraneous-dependencies */

let path = require('path');
let fs = require('fs/promises');
let glob = require('fast-glob');
let {copyTypes} = require('./copy-types.cjs');
let {sortPackageJson} = require('sort-package-json');

const EXCLUSIONS = [
  'atlaspack',
  '@atlaspack/parcel-to-atlaspack',
  '@atlaspack/bundler-experimental',
  '@atlaspack/rust',
  '@atlaspack/conditional-import-types',
  '@atlaspack/swc-plugin-contextual-imports',
  '@atlaspack/macros',
  '@atlaspack/validator-eslint',
  '@atlaspack/validator-typescript',
  '@atlaspack/create-react-app',
  '@atlaspack/query',
  '@atlaspack/bundle-stats',
  '@atlaspack/repl',
  '@atlaspack/resolver-repl-runtimes',
  '@atlaspack/lsp',
  '@atlaspack/lsp-protocol',
  '@atlaspack/reporter-lsp',
  '@atlaspack/apvm',
  '@atlaspack/apvm-linux-amd64',
  '@atlaspack/apvm-linux-arm64',
  '@atlaspack/apvm-macos-amd64',
  '@atlaspack/apvm-macos-arm64',
  '@atlaspack/apvm-windows-amd64',
  // Sass dep causes issues
  '@atlaspack/transformer-sass',
];
const entryDir = path.join(__dirname, '../entries');
const packagesDir = path.join(__dirname, '../../..');
const libDir = path.join(__dirname, '../lib');

const pluginPrefixes = [
  'transformer',
  'resolver',
  'bundler',
  'reporter',
  'runtime',
  'packager',
  'compressor',
  'namer',
  'optimizer',
  'config',
].map((pluginType) => `@atlaspack/${pluginType}-`);

async function getEntries() {
  let entries = await glob(`*/*/package.json`, {
    cwd: path.resolve('../..'),
    absolute: true,
  });

  return entries
    .map((packagePath) => ({packagePath, ...require(packagePath)}))
    .filter(
      ({name, private: isPrivate}) => !isPrivate && !EXCLUSIONS.includes(name),
    )
    .map(({name, source, atlaspackReferences, packagePath}) => {
      let entryName = name.substring(`@atlaspack/`.length);
      let isPlugin = pluginPrefixes.some((prefix) => name.startsWith(prefix));
      let references = atlaspackReferences
        ? glob.sync(atlaspackReferences, {
            cwd: path.dirname(packagePath),
            absolute: true,
          })
        : [];

      return {
        entryName,
        importSpecifier: source ? path.join(name, source) : name,
        pluginSpecifier: isPlugin ? name : null,
        references,
      };
    });
}

async function main() {
  const superPkgJson = JSON.parse(
    await fs.readFile(path.join(__dirname, '..', 'package.json'), 'utf8'),
  );

  superPkgJson.exports = {
    './*': {default: './*'},
    '.': {
      default: './lib/core.js',
      types: './types/@atlaspack/core/index.d.ts',
    },
  };
  let entries = await getEntries();

  // Add worker entries
  entries.push({
    entryName: 'worker',
    importSpecifier: '@atlaspack/core/src/worker',
  });
  entries.push({
    entryName: 'napi-worker',
    importSpecifier: '@atlaspack/core/src/atlaspack-v3/worker/napi-worker',
  });
  entries.push({
    entryName: 'ThreadsChild',
    importSpecifier: '@atlaspack/workers/src/threads/ThreadsChild',
  });
  entries.push({
    entryName: 'ProcessChild',
    importSpecifier: '@atlaspack/workers/src/process/ProcessChild',
  });

  let internalPlugins = [];

  for (let {
    importSpecifier,
    entryName,
    pluginSpecifier,
    references = [],
  } of entries) {
    let code = [];

    if (entryName === 'cli') {
      code.push(`import '${importSpecifier}'`);
    } else {
      code.push(
        `export * from '${importSpecifier}'`,
        `export {default} from '${importSpecifier}'`,
      );
    }
    if (pluginSpecifier) {
      internalPlugins.push([pluginSpecifier, `require('${importSpecifier}')`]);
    }

    let entryPath = path.join(entryDir, entryName + '.js');
    await writeFile(entryPath, code.join('\n'));
    superPkgJson.exports[`./${entryName}`] =
      superPkgJson.exports[`./${entryName}`] || {};
    superPkgJson.exports[`./${entryName}`].default = `./lib/${entryName}.js`;

    for (let reference of references) {
      let target = path
        .join(libDir, path.relative(packagesDir, reference))
        .replace('/src/', '/');

      await copyFile(reference, target);
    }
  }

  let internalPluginMap = internalPlugins
    .map(([key, value]) => `"${key}": () => ${value}`)
    .join(',');

  await writeFile(
    path.join(__dirname, '../patches', 'internal-plugins.js'),
    `export default {${internalPluginMap}}`,
  );

  copyTypes(superPkgJson);

  await fs.writeFile(
    path.join(__dirname, '..', 'package.json'),
    JSON.stringify(sortPackageJson(superPkgJson), null, 2),
    'utf8',
  );
}

async function copyFile(from, to) {
  await fs.mkdir(path.dirname(to), {recursive: true});
  await fs.copyFile(from, to);
}

async function writeFile(filePath, content) {
  await fs.mkdir(path.dirname(filePath), {recursive: true});
  await fs.writeFile(filePath, content, 'utf8');
}

main();
