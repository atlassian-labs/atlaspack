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
  // Sass dep causes issues
  '@atlaspack/transformer-sass',
];
const entryDir = path.join(__dirname, '../entries');

async function getEntries() {
  let entries = await glob(`*/*/package.json`, {
    cwd: path.resolve('../..'),
    absolute: true,
  });

  return entries
    .map((path) => require(path))
    .filter(({name, private}) => !private && !EXCLUSIONS.includes(name))
    .map(({name}) => {
      let entryName = name.substring(`@atlaspack`.length);

      return {entryName, importSpecifier: name};
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

  for (let {importSpecifier, entryName} of entries) {
    let code;
    if (entryName === 'cli') {
      code = `import '${importSpecifier}'`;
    } else {
      code = [
        `export * from '${importSpecifier}'`,
        `export {default} from '${importSpecifier}'`,
      ].join('\n');
    }

    let entryPath = path.join(entryDir, entryName + '.js');
    await fs.mkdir(path.dirname(entryPath), {recursive: true});
    await fs.writeFile(entryPath, code, {encoding: 'utf8'});
  }
}

main();
