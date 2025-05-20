import * as path from 'node:path';
import * as fs from 'node:fs';

export async function resolveTypesEntry(
  pkgDir: string,
): Promise<string | null> {
  const pkgJson = JSON.parse(
    await fs.promises.readFile(path.join(pkgDir, 'package.json'), 'utf8'),
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
