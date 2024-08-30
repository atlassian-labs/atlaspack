// @flow strict-local

import type {FileSystem} from '@atlaspack/rust';
import type {
  FilePath,
  FileSystem as ClassicFileSystem,
} from '@atlaspack/types-internal';

import {jsCallable} from './jsCallable';

// Move to @atlaspack/utils or a dedicated v3 / migration package later
export function toFileSystemV3(fs: ClassicFileSystem): FileSystem {
  return {
    canonicalize: jsCallable((path: FilePath) => fs.realpathSync(path)),
    createDirAll: jsCallable((path: FilePath) => fs.mkdirp(path)),
    cwd: jsCallable(() => fs.cwd()),
    readToString: jsCallable((path: string) => fs.readFileSync(path, 'utf8')),
    read: jsCallable((path: string) => fs.readFileSync(path)),
    exists: jsCallable((path: string) => fs.existsSync(path)),
    isFile: (path: string) => {
      try {
        return fs.statSync(path).isFile();
      } catch {
        return false;
      }
    },
    isDir: (path: string) => {
      try {
        return fs.statSync(path).isDirectory();
      } catch {
        return false;
      }
    },
  };
}
