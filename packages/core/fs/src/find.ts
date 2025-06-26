import type {FilePath, FileSystem} from '@atlaspack/types-internal';
import path from 'path';

export function findNodeModule(fs: FileSystem, moduleName: string, dir: FilePath): FilePath | null | undefined {
  let {root} = path.parse(dir);
  while (dir !== root) {
    // Skip node_modules directories
    if (path.basename(dir) === 'node_modules') {
      dir = path.dirname(dir);
    }

    try {
      let moduleDir = path.join(dir, 'node_modules', moduleName);
      let stats = fs.statSync(moduleDir);
      if (stats.isDirectory()) {
        return moduleDir;
      }
    } catch (err: any) {
      // ignore
    }

    // Move up a directory
    dir = path.dirname(dir);
  }

  return null;
}

export function findAncestorFile(fs: FileSystem, fileNames: Array<string>, dir: FilePath, root: FilePath): FilePath | null | undefined {
  let {root: pathRoot} = path.parse(dir);
  // eslint-disable-next-line no-constant-condition
  while (true) {
    if (path.basename(dir) === 'node_modules') {
      return null;
    }

    for (const fileName of fileNames) {
      let filePath = path.join(dir, fileName);
      try {
        if (fs.statSync(filePath).isFile()) {
          return filePath;
        }
      } catch (err: any) {
        // ignore
      }
    }

    if (dir === root || dir === pathRoot) {
      break;
    }

    dir = path.dirname(dir);
  }

  return null;
}

export function findFirstFile(fs: FileSystem, filePaths: Array<FilePath>): FilePath | null | undefined {
  for (let filePath of filePaths) {
    try {
      if (fs.statSync(filePath).isFile()) {
        return filePath;
      }
    } catch (err: any) {
      // ignore
    }
  }
}
