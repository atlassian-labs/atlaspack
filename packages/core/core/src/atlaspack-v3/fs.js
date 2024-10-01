// @flow strict-local

// import type { JsCallable } from "./jsCallable";
import type {FileSystem} from '@atlaspack/rust';
import type {
  Encoding,
  FilePath,
  FileSystem as ClassicFileSystem,
} from '@atlaspack/types';

import {jsCallable} from './jsCallable';

// Move to @atlaspack/utils or a dedicated v3 / migration package later
export function toFileSystemV3(fs: ClassicFileSystem): FileSystem {
  return {
    // $FlowFixMe migrate to TypeScript
    canonicalize: jsCallable((path: FilePath) => fs.realpathSync(path)),
    createDirectory: jsCallable((path: FilePath) => fs.mkdirp(path)),
    // $FlowFixMe migrate to TypeScript
    cwd: jsCallable(() => fs.cwd()),
    // $FlowFixMe migrate to TypeScript
    readFile: jsCallable((path: string, encoding?: Encoding) => {
      if (!encoding) {
        // $FlowFixMe
        return [...fs.readFileSync(path)];
      }
      return fs.readFileSync(path, encoding);
    }),
    // $FlowFixMe migrate to TypeScript
    isFile: jsCallable((path: string) => {
      try {
        return fs.statSync(path).isFile();
      } catch {
        return false;
      }
    }),
    // $FlowFixMe migrate to TypeScript
    isDir: jsCallable((path: string) => {
      try {
        return fs.statSync(path).isDirectory();
      } catch {
        return false;
      }
    }),
  };
}
