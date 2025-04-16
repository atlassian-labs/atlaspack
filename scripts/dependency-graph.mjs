// @ts-check
// node ./scripts/dependency-graph.mjs > atlaspack.dot

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import * as process from 'node:process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

const { workspaces } = JSON.parse(fs.readFileSync(path.join(__root, 'package.json'), 'utf8'))

let dot = ''
let packaages = {}

for (const workspace of workspaces) {
  for (const packageJson of glob.sync(path.join(workspace, 'package.json'), { cwd: __root })) {
    const { name, dependencies = {}, devDependencies = {} } = JSON.parse(fs.readFileSync(path.join(__root, packageJson), 'utf8'))

    for (const dependency of Object.keys({ ...dependencies, ...devDependencies })) {
      if (!dependency.startsWith('@atlaspack')) {
        continue
      }
      dot += `  "${name}" -> "${dependency}";\n`
      packaages[name] = true
      packaages[dependency] = true
    }
  }
}

dot = `digraph {\n  ${dot.trim()}\n}`
process.stdout.write(dot)
process.stderr.write(`\n\nNumber of packages: ${Object.keys(packaages).length}\n`)
