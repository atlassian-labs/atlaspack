// package.json#os and package.json#cpu cannot be specified in the workspace because it breaks
// the `yarn install`. To get around that, this prepare script is used to mutate the package.json
// to add them in

import * as fs from "node:fs";
import * as path from "node:path";
import * as process from "node:process";

const projectDir = process.cwd();
const packageJsonPath = path.join(projectDir, 'package.json')

if (!fs.existsSync(packageJsonPath)) {
  // eslint-disable-next-line no-console
  console.error(`DoesNotExist: ${packageJsonPath}`)
  process.exit(1)
}

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))


const [,platform, arch] = packageJson.name.split('/')[1].split('-')
if (!platform || !arch) {
  // eslint-disable-next-line no-console
  console.error(`IncorrectNameFormat: ${packageJson.name} should be named "project-platform-arch"`)
  process.exit(1)
}

const nodeArch = {
  'amd64': 'x64',
  'arm64': 'arm64',
}[arch];

const nodePlatform = {
  'linux': 'linux',
  'macos': 'darwin',
  'windows': 'win32',
}[platform];

if (!nodeArch || !nodePlatform) {
  // eslint-disable-next-line no-console
  console.error(`NoTarget: ${packageJson.name} has incorrect arch/platform specifiers`)
  // eslint-disable-next-line no-console
  console.error(`  needs: "amd64", "arm64", "linux", "macos", "windows"`)
  process.exit(1)
}

packageJson.os = [nodePlatform]
packageJson.cpu = [nodeArch]
packageJson.scripts = undefined

fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2), 'utf8')
