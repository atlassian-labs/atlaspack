// @ts-expect-error TS2305
import type {FileSystem, JsCallable} from '@atlaspack/rust';
import type {
  Encoding,
  FilePath,
  FileSystem as IFileSystem,
} from '@atlaspack/types';

import {jsCallable} from './jsCallable';

// @ts-expect-error TS2420
export class FileSystemV3 implements FileSystem {
  #fs: IFileSystem;

  constructor(fs: IFileSystem) {
    this.#fs = fs;
  }

  canonicalize: JsCallable<[FilePath], FilePath> = jsCallable(
    (path: FilePath) => this.#fs.realpathSync(path),
  );

  createDirectory: JsCallable<[FilePath], Promise<undefined>> = jsCallable(
    (path: FilePath) => this.#fs.mkdirp(path),
  );

  cwd: JsCallable<[], FilePath> = jsCallable(() => this.#fs.cwd());

  isFile: JsCallable<[FilePath], boolean> = (path: string) => {
    try {
      return this.#fs.statSync(path).isFile();
    } catch {
      return false;
    }
  };

  isDir: JsCallable<[FilePath], boolean> = (path: string) => {
    try {
      return this.#fs.statSync(path).isDirectory();
    } catch {
      return false;
    }
  };

  readFile: JsCallable<[FilePath, Encoding], string> = jsCallable(
    (path: string, encoding?: Encoding) => {
      if (!encoding) {
        return [...this.#fs.readFileSync(path)];
      } else {
        return this.#fs.readFileSync(path, encoding);
      }
    },
  );
}
