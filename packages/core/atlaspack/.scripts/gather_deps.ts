import * as fs from 'node:fs';

export async function gatherDependencies(
  ...targets: string[]
): Promise<Record<string, string>> {
  const results = {};

  for (const target of targets) {
    const file = await fs.promises.readFile(target, 'utf8');
    const pkg = JSON.parse(file);

    for (const [key, value] of Object.entries(pkg.dependencies || {})) {
      if (key.startsWith('@atlaspack/')) continue;
      results[key] = value;
    }
  }

  return results;
}

export async function gatherDevDependencies(
  ...targets: string[]
): Promise<Record<string, string>> {
  const results = {};

  for (const target of targets) {
    const file = await fs.promises.readFile(target, 'utf8');
    const pkg = JSON.parse(file);

    for (const [key, value] of Object.entries(pkg.devDependencies || {})) {
      if (key.startsWith('@atlaspack/')) continue;
      results[key] = value;
    }
  }

  return results;
}
