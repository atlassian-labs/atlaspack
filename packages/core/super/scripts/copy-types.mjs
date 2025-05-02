/* eslint-disable import/no-extraneous-dependencies */

// This will copy types over to the `/types` folder and rewrite imports to relative paths

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __types = path.normalize(path.join(__dirname, '..', 'types'));
// const __root = path.normalize(path.join(__dirname, '..', '..', '..', '..'));

if (fs.existsSync(__types)) {
  fs.rmSync(__types, {recursive: true, force: true});
}
fs.mkdirSync(__types);

const superPkgJson = JSON.parse(
  fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'),
);

const typeEntriesMap = {}

for (const copyType of superPkgJson.copyTypes) {
  const packageJsonPath = findPackageJson(copyType, __dirname)
  if (!packageJsonPath) continue

  const typesDir = path.join(__types, copyType)
  const pkgDir = path.dirname(packageJsonPath)

  const entry = resolveTypesEntry(pkgDir)
  typeEntriesMap[copyType] = entry

  const found = glob.sync('**/*.d.ts', {
    cwd: pkgDir,
    ignore: ['**/node_modules/**'],
  });

  for (const entry of found) {
    if (!fs.existsSync(path.dirname(path.join(typesDir, entry)))) {
      fs.mkdirSync(path.dirname(path.join(typesDir, entry)), { recursive: true });
    }
    fs.copyFileSync(
      path.join(pkgDir, entry),
      path.join(typesDir, entry)
    )
  }
}

const found = glob.sync('**/*.d.ts', { cwd: __types });
for (const entry of found) {
  let entryPath = path.join(__types, entry);
  let entryDir = path.dirname(entryPath);

  let content = fs.readFileSync(entryPath, 'utf8');

  for (const [sourceKey, typesPath] of Object.entries(typeEntriesMap)) {
    const newPath = path.join(__types, sourceKey, typesPath)
    const newPathRel = path.relative(entryDir, newPath)
    content = content
      .replaceAll(`'${sourceKey}'`, `'${newPathRel}'`)
      .replaceAll(`"${sourceKey}"`, `"${newPathRel}"`);
  }

  if (process.env.DEBUG_TYPES === "true") {
    // eslint-disable-next-line no-useless-escape
    const imports_re = /import(?:(?:(?:[ \n\t]+([^ *\n\t\{\},]+)[ \n\t]*(?:,|[ \n\t]+))?([ \n\t]*\{(?:[ \n\t]*[^ \n\t"'\{\}]+[ \n\t]*,?)+\})?[ \n\t]*)|[ \n\t]*\*[ \n\t]*as[ \n\t]+([^ \n\t\{\}]+)[ \n\t]+)from[ \n\t]*(?:['"])([^'"\n]+)(['"])/g

    for (const match of Array.from(matchAll(content, imports_re))) {
      const specifier = match[4]
      if (specifier.startsWith(".")) continue
      // eslint-disable-next-line no-console
      console.log(`Missing types for: "${match[4]}" in ${entryPath}`)
    }
  }

  fs.writeFileSync(entryPath, content, 'utf8')
}

function resolveTypesEntry(pkgDir) {
  const pkgJson = JSON.parse(
    fs.readFileSync(path.join(pkgDir, 'package.json'), 'utf8'),
  );

  if (pkgJson.types) {
    return pkgJson.types
  }

  if (pkgJson.typings) {
    return pkgJson.typings
  }

  if (
    fs.existsSync(path.join(pkgDir, 'index.d.ts'))
  ) {
    return 'index.d.ts'
  }

  if (pkgJson.main) {
    let possibleTyping = pkgJson.main.replace(".js", '.d.ts')
    if (
      fs.existsSync(path.join(pkgDir, possibleTyping))
    ) {
      return possibleTyping
    }
  }

  return null
}

function findPackageJson(specifier, cwd) {
  let current = cwd

  // eslint-disable-next-line no-constant-condition
  while (true) {
    const test = path.join(current, 'node_modules', specifier, 'package.json')
    if (fs.existsSync(test)) {
      return test
    }

    const next = path.dirname(current)
    if (next === current) {
      break
    }
    current = next
  }

  return null
}

function* matchAll(str, regexp) {
  const flags = regexp.global ? regexp.flags : regexp.flags + "g";
  const re = new RegExp(regexp, flags);
  let match;
  // eslint-disable-next-line no-cond-assign
  while (match = re.exec(str)) {
    yield match;
  }
}
