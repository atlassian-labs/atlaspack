// @flow strict-local

import type {FileSystem} from '@atlaspack/rust';
import type {
  Encoding,
  FilePath,
  FileSystem as ClassicFileSystem,
} from '@atlaspack/types-internal';

import {jsCallable} from './jsCallable';

// Move to @atlaspack/utils or a dedicated v3 / migration package later
export function toFileSystemV3(fs: ClassicFileSystem): FileSystem {
  return {
    canonicalize: jsCallable((path: FilePath) => fs.realpathSync(path)),
    createDirectory: jsCallable((path: FilePath) => fs.mkdirp(path)),
    cwd: jsCallable(() => fs.cwd()),
    readFile: jsCallable((path: string, encoding?: Encoding) => {
      if (!encoding) {
        // $FlowFixMe
        return [...fs.readFileSync(path)];
      } else {
        return fs.readFileSync(path, encoding);
      }
    }),
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
