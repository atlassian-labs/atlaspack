let path = require('path');
let fs = require('fs/promises');
let glob = require('fast-glob');

const EXCLUSIONS = [
  '@atlaspack/super',
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
].map((pluginType) => `@atlaspack/${pluginType}-`);

async function getEntries() {
  let entries = await glob(`*/*/package.json`, {
    cwd: path.resolve('../..'),
    absolute: true,
  });

  return entries
    .map((packagePath) => ({packagePath, ...require(packagePath)}))
    .filter(({name, private}) => !private && !EXCLUSIONS.includes(name))
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
  let entries = await getEntries();

  // Add worker entries
  entries.push({
    entryName: 'worker',
    importSpecifier: '@atlaspack/core/src/worker',
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
    path.join(entryDir, 'internal-plugins.js'),
    `export default {${internalPluginMap}}`,
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
