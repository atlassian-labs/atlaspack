import * as path from 'node:path';
import * as fs from 'node:fs';
import {spawn} from './spawn.mts';

export async function findPackageJson(
  specifier: string,
  cwd: string,
): Promise<string | null> {
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
