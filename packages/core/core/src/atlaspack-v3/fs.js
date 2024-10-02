// @flow strict-local

import type {FileSystem} from '@atlaspack/rust';
import type {
  Encoding,
  FilePath,
  FileSystem as ClassicFileSystem,
} from '@atlaspack/types';
import type {JsCallable} from './jsCallable';
import {jsCallable} from './jsCallable';

export class NativeFileSystem implements FileSystem {
  #fs: ClassicFileSystem;

  constructor(fs: ClassicFileSystem) {
    this.#fs = fs;
  }

  canonicalize: JsCallable<[FilePath], FilePath> = jsCallable(path =>
    this.#fs.realpathSync(path),
  );

  createDirectory: JsCallable<[FilePath], Promise<void>> = jsCallable(path =>
    this.#fs.mkdirp(path),
  );

  cwd: JsCallable<[], FilePath> = jsCallable(() => this.#fs.cwd());

  isDir: JsCallable<[FilePath], boolean> = jsCallable((path: string) => {
    try {
      return this.#fs.statSync(path).isDirectory();
    } catch {
      return false;
    }
  });

  isFile: JsCallable<[FilePath], boolean> = jsCallable(path => {
    try {
      return this.#fs.statSync(path).isDirectory();
    } catch {
      return false;
    }
  });

  readFile: JsCallable<[FilePath, Encoding | void], string | Array<number>> =
    jsCallable((path, encoding) => {
      if (!encoding) {
        // $FlowFixMe
        return [...this.#fs.readFileSync(path)];
      }
      return this.#fs.readFileSync(path, encoding);
    });
}
