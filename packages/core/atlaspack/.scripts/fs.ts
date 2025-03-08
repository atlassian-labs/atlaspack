import * as path from 'node:path';
import * as fs from 'node:fs';
import {localResolutions, packageMappings} from './package-imports.ts';

export async function copyFile(options: {
  from: string | string[];
  to?: string;
  dir?: string;
  transformImports?: boolean;
}) {
  if (options.to) {
    if (Array.isArray(options.from)) return;
    await fs.promises.cp(options.from, options.to, {
      recursive: true,
      force: true,
    });
    if (options.transformImports) {
      let data = await fs.promises.readFile(options.to, 'utf8');
      for (const x of Object.entries(localResolutions)) {
        data = data.replaceAll(x[0], x[1] as string);
      }
      await fs.promises.writeFile(options.to, data, 'utf8');
    }
    return;
  }
  if (!options.dir) return;

  let from: string[] = Array.isArray(options.from)
    ? options.from
    : [options.from];

  for (const target of from) {
    const output = path.join(options.dir, path.basename(target));
    await fs.promises.cp(target, output, {
      recursive: true,
      force: true,
    });
    if ((await fs.promises.stat(output)).isFile() && options.transformImports) {
      let data = await fs.promises.readFile(output, 'utf8');
      for (const x of Object.entries(localResolutions)) {
        data = data.replaceAll(x[0], x[1] as string);
      }
      await fs.promises.writeFile(output, data, 'utf8');
    }
  }
}

export async function writeFile(target: string, content: string) {
  await fs.promises.writeFile(target, content, 'utf8');
}

export async function remapImports(...targetPaths: string[]) {
  for (const targetPath of targetPaths) {
    let data = await fs.promises.readFile(targetPath, 'utf8');
    for (const x of Object.entries(localResolutions)) {
      data = data.replaceAll(x[0], x[1] as string);
    }
    for (const [before, after] of packageMappings) {
      data = data.replaceAll(before, after);
    }
    await fs.promises.writeFile(targetPath, data, 'utf8');
  }
}

export async function rm(...targets: string[]): Promise<void> {
  for (const target of targets) {
    await fs.promises.rm(target, {recursive: true, force: true});
  }
}

export async function cp(source: string, target: string): Promise<void> {
  await fs.promises.cp(source, target, {recursive: true, force: true});
}

export async function readJson<T = any>(target: string): Promise<T> {
  const file = await fs.promises.readFile(target, 'utf8');
  return JSON.parse(file);
}

export async function writeJson<T = any>(
  target: string,
  data: any,
): Promise<void> {
  await fs.promises.writeFile(target, JSON.stringify(data, null, 2), 'utf8');
}
