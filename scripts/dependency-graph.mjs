/* eslint-disable no-unused-vars */
/* eslint-disable no-console */
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

// const { workspaces } = JSON.parse(fs.readFileSync(path.join(__root, 'package.json'), 'utf8'))

// let allDeps = {}
// let allDevDeps = {}

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

// for (const packagee of fs.readdirSync(path.join(__root, 'packages', 'shims'))) {
//   console.log(packagee)
//   const packagePath = path.join(__root, 'packages', 'shims', packagee)
//   const packageJsonPath = path.join(packagePath, 'package.json')
//   if (!fs.existsSync(packageJsonPath)) continue
//   const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))

//   const newPackageJson = {
//     name: packageJson.name,
//     description: packageJson.description,
//     version: packageJson.version,
//     license: packageJson.license,
//     type: "commonjs",
//     main: "./index.js",
//     types: "./index.d.ts",
//     publishConfig: {
//       access: 'public'
//     },
//     repository:  {
//       "type": "git",
//       "url": "https://github.com/atlassian-labs/atlaspack.git"
//     },
//   }

//   fs.writeFileSync(packageJsonPath, JSON.stringify(newPackageJson, null, 2))

//   fs.rmSync(path.join(packagePath, 'index.js'), { recursive: true, force: true })
//   fs.rmSync(path.join(packagePath, 'index.d.ts'), { recursive: true, force: true })
//   fs.rmSync(path.join(packagePath, 'index.flow'), { recursive: true, force: true })

//   let [namespce, pkg] = packagee.split(/-(.*)/s).filter(v => v)
//   if (namespce === 'core' || namespce === 'utils') {
//     namespce = ''
//   } else {
//     namespce = `${namespce}s/`
//   }
//   fs.writeFileSync(path.join(packagePath, 'index.js'), `module.exports = require('atlaspack/${namespce}${pkg}/index.js');\n`)
//   fs.writeFileSync(path.join(packagePath, 'index.d.ts'), `export * from 'atlaspack/${namespce}${pkg}/index.js';\nexport {default} from 'atlaspack/${namespce}${pkg}/index.js';\n`)
//   fs.writeFileSync(path.join(packagePath, 'index.js.flow'), `
// /* eslint-disable import/no-extraneous-dependencies */
// /* eslint-disable monorepo/no-internal-import */
// // @flow
// export * from 'atlaspack/src/${namespce}${pkg}/index.js';
// // $FlowFixMe
// export {default} from 'atlaspack/src/${namespce}${pkg}/index.js';`.trim() + '\n')

//   child_process.execSync('/usr/bin/env node /run/user/1000/fnm_multishells/212082_1745134745812/bin/sort-package-json', {
//     cwd: packagePath
//   })
// }

const destPath = '/Users/dalsh/Development/atlassian-labs/atlaspack/packages/core/atlaspack/src'
const subfolder = 'compressors'

for (const packageName of fs.readdirSync(path.join(__root, 'packages', subfolder))) {
  console.log(packageName)
  const packagePath = path.join(__root, 'packages', subfolder, packageName)
  const packageJsonPath = path.join(packagePath, 'package.json')

  if (!fs.existsSync(packageJsonPath)) continue
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))
  const packageJsonName = packageJson.name.split('@atlaspack/')[1]

  const packagePathDest = path.join(__root, 'packages', 'shims', `${subfolder}-${packageName}`)

  const packagePathCoreDest = path.join(destPath, 'compressors', packageName)
  console.log(packageName, packageName)

  fs.rmSync(packagePathDest, { force: true, recursive: true })
  fs.renameSync(packagePath, packagePathDest)
  fs.renameSync(path.join(packagePathDest, 'src'), packagePathCoreDest)

  if (fs.existsSync(path.join(packagePathDest, 'test'))) for (const entry of fs.readdirSync(path.join(packagePathDest, 'test'))) {
    fs.renameSync(path.join(packagePathDest, 'test', entry), path.join(packagePathCoreDest, entry))
  }

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

  const entry = path.basename(packageJson.source)
  if (entry !== 'index.js') {
    fs.writeFileSync(path.join(packagePathCoreDest, 'index.js'), `
  // @flow
  export * from './${entry}';
  export {default} from './${entry}';
    `.trim() + '\n')
  }

  fs.writeFileSync(path.join(packagePathDest, 'package.json'), JSON.stringify(newPackageJson, null, 2))

  fs.rmSync(path.join(packagePathDest, 'index.js'), { recursive: true, force: true })
  fs.rmSync(path.join(packagePathDest, 'index.d.ts'), { recursive: true, force: true })
  fs.rmSync(path.join(packagePathDest, 'index.flow'), { recursive: true, force: true })

  let [namespce, pkg] = packageName.replace('@atlassian').split(/-(.*)/s).filter(v => v)

  fs.writeFileSync(path.join(packagePathDest, 'index.js'), `module.exports = require('atlaspack/${subfolder}/${packageName}/index.js');\n`)
  fs.writeFileSync(path.join(packagePathDest, 'index.d.ts'), `export * from 'atlaspack/${subfolder}/${packageName}/index.js';\nexport {default} from 'atlaspack/${subfolder}/${packageName}/index.js';\n`)
  fs.writeFileSync(path.join(packagePathDest, 'index.js.flow'), `
/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable monorepo/no-internal-import */
// @flow
export * from 'atlaspack/src/${subfolder}/${packageName}/index.js';
// $FlowFixMe
export {default} from 'atlaspack/src/${subfolder}/${packageName}/index.js';`.trim() + '\n')

  child_process.execSync('/usr/bin/env node /Users/dalsh/Library/Caches/fnm_multishells/79866_1745157444054/bin/sort-package-json', {
    cwd: packagePathDest
  })

  break
}
