/* eslint-disable no-console */
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as url from 'node:url';
import * as child_process from 'node:child_process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);
const __src = path.join(__root, 'packages', 'unified', 'src');
const __dist = path.join(__root, 'packages', 'unified', 'lib');
const __vendor = path.join(__src, 'vendor');
const __vendorDist = path.join(__dist, 'vendor');

void (async function main() {
  if (fs.existsSync(__vendorDist)) {
    await fs.promises.rm(__vendorDist, {recursive: true, force: true});
  }
  await fs.promises.mkdir(__vendorDist, {recursive: true});

  const includedPackages = {};

  async function includePackage(packageName, cwd) {
    const pkgJsonPath = await findPackageJson(packageName, cwd);
    const pkgJson = JSON.parse(await fs.promises.readFile(pkgJsonPath, 'utf8'));
    const pkgJsonDir = path.dirname(pkgJsonPath);
    includedPackages[`${pkgJson.name}@${pkgJson.version}`] = {
      name: pkgJson.name,
      version: pkgJson.version,
      main: url
        .fileURLToPath(
          import.meta.resolve(pkgJson.name, url.pathToFileURL(cwd)),
        )
        .replace(pkgJsonDir + path.sep, ''),
      types: resolveTypesEntry(pkgJsonDir),
      path: pkgJsonDir,
      from: cwd,
      dependencies: {},
    };

    for (const dependency of Object.keys(pkgJson.dependencies || {})) {
      includedPackages[`${pkgJson.name}@${pkgJson.version}`].dependencies[
        dependency
      ] = await includePackage(dependency, pkgJsonDir);
    }

    return pkgJson.version;
  }

  for (const foundRel of glob.sync('**/vendor.json', {
    cwd: __vendor,
  })) {
    const foundDir = path.dirname(path.join(foundRel));
    const meta = JSON.parse(
      await fs.promises.readFile(path.join(__vendor, foundRel), 'utf8'),
    );
    await includePackage(meta.name, path.join(__vendor, foundDir));
  }

  for (const included of Object.values(includedPackages)) {
    if (included.from.startsWith(__vendor)) {
      await cpDir(included.path, path.join(__vendorDist, included.name));

      if (!fs.existsSync(path.join(__vendorDist, included.name, 'index.js'))) {
        await fs.promises.writeFile(
          path.join(__vendorDist, included.name, 'index.js'),
          `module.exports = require('./${included.main}')`,
        );
      }
      if (
        !fs.existsSync(path.join(__vendorDist, included.name, 'index.d.ts'))
      ) {
        await fs.promises.writeFile(
          path.join(__vendorDist, included.name, 'index.d.ts'),
          [
            '// @ts-ignore',
            `export * from './${included.types}';`,
            '// @ts-ignore',
            `export type * from './${included.types}';`,
            '// @ts-ignore',
            `export {default} from './${included.types}';`,
          ].join('\n'),
        );
      }
    } else {
      await cpDir(
        included.path,
        path.join(__vendorDist, `${included.name}@${included.version}`),
      );
    }

    for (const [key, value] of Object.entries(included.dependencies)) {
      const pathToDepAbs = path.join(__vendorDist, `${key}@${value}`);

      for (const foundRel of glob.sync('**/*', {
        cwd: path.join(__vendorDist),
        ignore: ['**/*.json'],
      })) {
        const found = path.join(path.join(__vendorDist, foundRel));
        if (!(await fs.promises.stat(found)).isFile()) {
          continue;
        }
        const pathToDep = path.relative(path.dirname(found), pathToDepAbs);
        let content = renameImports(
          await fs.promises.readFile(found, 'utf8'),
          key,
          pathToDep,
        );
        await fs.promises.writeFile(found, content, 'utf8');
      }
    }
  }
})();

async function findPackageJson(specifier, cwd) {
  // const require = module.createRequire(cwd);
  // const main = require.resolve(specifier)
  const main = await spawn(
    'node',
    ['-e', `"console.log(require.resolve('${specifier}'))"`],
    {
      cwd,
      shell: true,
    },
  );

  let current = main;

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const test = path.join(current, 'package.json');
    if (
      fs.existsSync(test) &&
      JSON.parse(fs.readFileSync(test, 'utf8')).name === specifier
    ) {
      return test;
    }

    const next = path.dirname(current);
    if (next === current) {
      break;
    }
    current = next;
  }

  return null;
}

function resolveTypesEntry(pkgDir) {
  const pkgJson = JSON.parse(
    fs.readFileSync(path.join(pkgDir, 'package.json'), 'utf8'),
  );

  if (pkgJson.types) {
    return pkgJson.types;
  }

  if (pkgJson.typings) {
    return pkgJson.typings;
  }

  if (fs.existsSync(path.join(pkgDir, 'index.d.ts'))) {
    return 'index.d.ts';
  }

  if (pkgJson.main) {
    let possibleTyping = pkgJson.main.replace('.js', '.d.ts');
    if (fs.existsSync(path.join(pkgDir, possibleTyping))) {
      return possibleTyping;
    }
  }

  return null;
}

async function cpDir(source, target) {
  if (!fs.existsSync(path.dirname(target))) {
    await fs.promises.mkdir(path.dirname(target), {recursive: true});
  }
  await fs.promises.cp(source, target, {recursive: true});

  if (path.join(target, 'node_modules')) {
    await fs.promises.rm(path.join(target, 'node_modules'), {
      recursive: true,
      force: true,
    });
  }
}

function spawn(cmd, args, options) {
  return new Promise((resolve, reject) => {
    const cp = child_process.spawn(cmd, args, options);
    const error = [];
    const stdout = [];
    cp.stdout.on('data', (data) => {
      stdout.push(data.toString());
    });

    cp.on('error', (e) => {
      error.push(e.toString());
    });

    cp.on('close', () => {
      if (error.length) reject(error.join(''));
      else resolve(stdout.join(''));
    });
  });
}

const re = {
  dynamic: /((import|require)\s*\(\s*('|"|`))(.*)(('|"|`)\s*\))/g,
  static: /(import\s*(\w+)*('|"|`))(.*)(('|"|`))/g,
  named: /((import|export)\s*(.*)from\s*('|"|`))(.*)(('|"|`))/g,
};

function subpath(input, to) {
  const arr = input.split('/');
  if (input.startsWith('@')) {
    arr.shift();
  }
  arr[0] = to;
  return arr.join('/');
}

function renameImports(contents, from, to) {
  contents = contents.replace(re.dynamic, (...match) => {
    if (match[4].startsWith(from)) {
      return match[1] + subpath(match[4], to) + match[5];
    } else {
      return match[0];
    }
  });

  contents = contents.replace(re.static, (...match) => {
    if (match[4].startsWith(from)) {
      return match[1] + subpath(match[4], to) + match[6];
    } else {
      return match[0];
    }
  });

  contents = contents.replace(re.named, (...match) => {
    if (match[5].startsWith(from)) {
      return match[1] + subpath(match[5], to) + match[7];
    } else {
      return match[0];
    }
  });

  return contents;
}
