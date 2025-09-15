/* eslint-disable no-console */
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as url from 'node:url';
import glob from 'glob';

const dirname = path.dirname(url.fileURLToPath(import.meta.url));
const root = path.dirname(dirname);

const rootPkg = JSON.parse(
  fs.readFileSync(path.join(root, 'package.json'), 'utf8'),
);

for (const workspace of rootPkg.workspaces) {
  for (const pkgPath of glob.sync(path.join(workspace, 'package.json'), {
    cwd: root,
  })) {
    const abs = path.join(root, pkgPath);
    const pkg = JSON.parse(fs.readFileSync(abs, 'utf8'));
    if (!pkg.types) continue;
    if (!pkg.source) continue;
    if (pkg.source != pkg.types) continue;

    pkg.types = pkg.types
      .replace('./src/', './lib/types/')
      .replace('.ts', '.d.ts');

    fs.writeFileSync(abs, JSON.stringify(pkg, null, 2), 'utf8');
  }
}
