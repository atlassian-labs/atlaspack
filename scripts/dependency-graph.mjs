// @ts-check
// node ./scripts/dependency-graph.mjs > atlaspack.dot

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import * as process from 'node:process';
import * as child_process from 'node:child_process';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);

const { workspaces } = JSON.parse(fs.readFileSync(path.join(__root, 'package.json'), 'utf8'))

let allDeps = {}
let allDevDeps = {}

// for (const workspace of workspaces) {
//   for (const packageJson of glob.sync(path.join(workspace, 'package.json'), { cwd: __root })) {
//     const { name, dependencies = {}, devDependencies = {} } = JSON.parse(fs.readFileSync(path.join(__root, packageJson), 'utf8'))
//     allDeps = {
//       ...allDeps,
//       ...dependencies,
//     }

//     allDevDeps = {
//       ...allDevDeps,
//       ...devDependencies,
//     }
//   }
// }


for (const packagee of fs.readdirSync(path.join(__root, 'packages', 'shims'))) {
  console.log(packagee)
  const packagePath = path.join(__root, 'packages', 'shims', packagee)
  const packageJsonPath = path.join(packagePath, 'package.json')
  if (!fs.existsSync(packageJsonPath)) continue
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))

  const newPackageJson = {
    name: packageJson.name,
    description: packageJson.description,
    version: packageJson.version,
    license: packageJson.license,
    main: "./index.js",
    types: "./index.d.ts",
    publishConfig: {
      access: 'public'
    },
    repository:  {
      "type": "git",
      "url": "https://github.com/atlassian-labs/atlaspack.git"
    },
    type: "commonjs",
  }

  fs.writeFileSync(packageJsonPath, JSON.stringify(newPackageJson, null, 2))

  fs.rmSync(path.join(packagePath, 'index.js'), { recursive: true, force: true })
  fs.rmSync(path.join(packagePath, 'index.d.ts'), { recursive: true, force: true })

  const [namespce, pkg] = packagee.split(/-(.*)/s).filter(v => v)
  fs.writeFileSync(path.join(packagePath, 'index.js'), `module.exports = require('atlaspack/${namespce}/${pkg}/index.js');\n`)
  fs.writeFileSync(path.join(packagePath, 'index.d.ts'), `export * from 'atlaspack/${namespce}/${pkg}/index.js';\nexport {default} from 'atlaspack/${namespce}/${pkg}/index.js';\n`)
  fs.writeFileSync(path.join(packagePath, 'index.flow'), `// @flow\nexport * from 'atlaspack/${namespce}/${pkg}/index.js';\nexport {default} from 'atlaspack/${namespce}/${pkg}/index.js';\n`)

  child_process.execSync('/usr/bin/env node /run/user/1000/fnm_multishells/212082_1745134745812/bin/sort-package-json', {
    cwd: packagePath
  })
}
