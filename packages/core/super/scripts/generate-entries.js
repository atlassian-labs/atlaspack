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
  // Sass dep causes issues
  '@atlaspack/transformer-sass',
];
const entryDir = path.join(__dirname, '../entries');

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
    .map((path) => require(path))
    .filter(({name, private}) => !private && !EXCLUSIONS.includes(name))
    .map(({name, source}) => {
      let entryName = name.substring(`@atlaspack/`.length);
      let isPlugin = pluginPrefixes.some((prefix) => name.startsWith(prefix));

      return {
        entryName,
        importSpecifier: source ? path.join(name, source) : name,
        pluginSpecifier: isPlugin ? name : null,
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

  for (let {importSpecifier, entryName, pluginSpecifier} of entries) {
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
    await fs.mkdir(path.dirname(entryPath), {recursive: true});
    await fs.writeFile(entryPath, code.join('\n'), {encoding: 'utf8'});
  }

  let internalPluginMap = internalPlugins
    .map(([key, value]) => `"${key}": () => ${value}`)
    .join(',');

  await fs.writeFile(
    path.join(entryDir, 'internal-plugins.js'),
    `export default {${internalPluginMap}}`,
  );
}

main();
