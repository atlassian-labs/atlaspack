import * as path from 'node:path';
import * as fs from 'node:fs';

export async function cpAll(source: string, target: string) {
  if (!fs.existsSync(path.dirname(target))) {
    await fs.promises.mkdir(path.dirname(target), {recursive: true});
  }
  await fs.promises.cp(source, target, {recursive: true});
}

export async function recreateDirAll(target: string): Promise<void> {
  if (fs.existsSync(target)) {
    await fs.promises.rm(target, {recursive: true, force: true});
  }
  await fs.promises.mkdir(target, {recursive: true});
}

export async function createDirAll(target: string): Promise<void> {
  if (!fs.existsSync(target)) {
    await fs.promises.mkdir(target, {recursive: true});
  }
}

export async function rm(target: string): Promise<void> {
  if (fs.existsSync(target)) {
    await fs.promises.rm(target, {recursive: true, force: true});
  }
}

export async function readToString(target: string): Promise<string> {
  return await fs.promises.readFile(target, 'utf8');
}

export async function writeString(
  target: string,
  contents: string,
): Promise<void> {
  await fs.promises.writeFile(target, contents, 'utf8');
}

export async function readJson<T extends {[key: string]: any}>(
  target: string,
): Promise<T> {
  return JSON.parse(await readToString(target));
}

export async function writeJson(target: string, contents: any): Promise<void> {
  function replacer(_key: string, value: any) {
    if (value instanceof Map) {
      const items = {};
      for (const [k, v] of Object.entries(value)) {
        items[k] = v;
      }
      return items;
    } else if (value instanceof Set) {
      return Array.from(value);
    } else {
      return value;
    }
  }
  await writeString(target, JSON.stringify(contents, replacer, 2));
}

export async function isFile(target: string): Promise<boolean> {
  try {
    return (await fs.promises.stat(target)).isFile();
  } catch (error) {
    return false;
  }
}
