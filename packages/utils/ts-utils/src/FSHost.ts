import type {FileSystem} from '@atlaspack/fs';
import type {FilePath} from '@atlaspack/types';
import TypeScriptModule from 'typescript'; // eslint-disable-line import/no-extraneous-dependencies
import path from 'path';

export class FSHost {
  fs: FileSystem;
  // @ts-expect-error - TS2709 - Cannot use namespace 'TypeScriptModule' as a type.
  ts: TypeScriptModule;

  // @ts-expect-error - TS2709 - Cannot use namespace 'TypeScriptModule' as a type.
  constructor(fs: FileSystem, ts: TypeScriptModule) {
    this.fs = fs;
    this.ts = ts;
  }

  getCurrentDirectory: () => FilePath = () => {
    return this.fs.cwd();
  };

  fileExists(filePath: FilePath): boolean {
    try {
      return this.fs.statSync(filePath).isFile();
    } catch (err: any) {
      return false;
    }
  }

  readFile(filePath: FilePath): undefined | string {
    try {
      return this.fs.readFileSync(filePath, 'utf8');
    } catch (err: any) {
      if (err.code === 'ENOENT') {
        return undefined;
      }

      throw err;
    }
  }

  directoryExists(filePath: FilePath): boolean {
    try {
      return this.fs.statSync(filePath).isDirectory();
    } catch (err: any) {
      return false;
    }
  }

  realpath(filePath: FilePath): FilePath {
    try {
      return this.fs.realpathSync(filePath);
    } catch (err: any) {
      return filePath;
    }
  }

  getAccessibleFileSystemEntries(dirPath: FilePath): {
    directories: Array<FilePath>;
    files: Array<FilePath>;
  } {
    try {
      let entries = this.fs.readdirSync(dirPath || '.').sort();
      let files: Array<FilePath> = [];
      let directories: Array<FilePath> = [];
      for (let entry of entries) {
        let filePath = path.join(dirPath, entry);

        let stat;
        try {
          stat = this.fs.statSync(filePath);
        } catch (e: any) {
          continue;
        }

        if (stat.isFile()) {
          files.push(entry);
        } else if (stat.isDirectory()) {
          directories.push(entry);
        }
      }

      return {files, directories};
    } catch (err: any) {
      return {files: [], directories: []};
    }
  }

  readDirectory(
    root: FilePath,
    extensions?: ReadonlyArray<string>,
    excludes?: ReadonlyArray<string>,
    includes?: ReadonlyArray<string>,
    depth?: number,
  ): any {
    return this.ts.matchFiles(
      root,
      extensions,
      excludes,
      includes,
      this.ts.sys.useCaseSensitiveFileNames,
      this.getCurrentDirectory(),
      depth,
      // @ts-expect-error - TS7006 - Parameter 'dirPath' implicitly has an 'any' type.
      (dirPath) => this.getAccessibleFileSystemEntries(dirPath),
      // @ts-expect-error - TS7006 - Parameter 'filePath' implicitly has an 'any' type.
      (filePath) => this.realpath(filePath),
      // @ts-expect-error - TS7006 - Parameter 'dirPath' implicitly has an 'any' type.
      (dirPath) => this.directoryExists(dirPath),
    );
  }
}
