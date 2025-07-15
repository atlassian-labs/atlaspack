import type {FileSystem} from '@atlaspack/fs';
import type {FilePath} from '@atlaspack/types';
import TypeScriptModule from 'typescript'; // eslint-disable-line import/no-extraneous-dependencies
import path from 'path';

export class FSHost {
  fs: FileSystem;
  ts: TypeScriptModule;

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
    } catch {
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
    } catch {
      return false;
    }
  }

  realpath(filePath: FilePath): FilePath {
    try {
      return this.fs.realpathSync(filePath);
    } catch {
      return filePath;
    }
  }

  getAccessibleFileSystemEntries(dirPath: FilePath): {
    directories: Array<FilePath>;
    files: Array<FilePath>;
  } {
    try {
      let entries = this.fs.readdirSync(dirPath || '.').sort();
      let files: Array<never> = [];
      let directories: Array<never> = [];
      for (let entry of entries) {
        let filePath = path.join(dirPath, entry);

        let stat;
        try {
          stat = this.fs.statSync(filePath);
        } catch {
          continue;
        }

        if (stat.isFile()) {
          files.push(entry);
        } else if (stat.isDirectory()) {
          directories.push(entry);
        }
      }

      return {files, directories};
    } catch {
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
      (dirPath) => this.getAccessibleFileSystemEntries(dirPath),
      (filePath) => this.realpath(filePath),
      (dirPath) => this.directoryExists(dirPath),
    );
  }
}
