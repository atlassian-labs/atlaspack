/* eslint-disable no-console */
import * as fs from 'node:fs';
import * as url from 'node:url';
import * as path from 'node:path';
import glob from 'glob';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.join(__dirname, '..');

const IGNORED_PATTERNS = [
  'core/atlaspack',
  'apvm',
  'node_modules',
  'node-resolver-core/test/fixture',
  'test/fixtures',
  'examples',
  'integration-tests',
  'workers/test/integration',
];

for (const packageJsonPathRel of glob.sync('packages/**/*/package.json', {
  cwd: __root,
})) {
  if (
    IGNORED_PATTERNS.some((pattern) => packageJsonPathRel.includes(pattern))
  ) {
    continue;
  }

  console.log(packageJsonPathRel);
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPathRel, 'utf8'));
  if (packageJson.types && !packageJson.types.endsWith('.d.ts')) {
    console.log(packageJson.types);
    const newType = packageJson.types
      .replace('src', 'lib/types')
      .replace('.ts', '.d.ts');
    console.log(newType);
    packageJson.types = newType;
    fs.writeFileSync(packageJsonPathRel, JSON.stringify(packageJson, null, 2));
  }
}
