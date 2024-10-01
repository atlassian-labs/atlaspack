import type {FileSystem} from '@atlaspack/fs';
import type {FilePath} from '@atlaspack/types';
import TypeScriptModule from 'typescript'; // eslint-disable-line import/no-extraneous-dependencies
import type {ParseConfigHost as IParseConfigHost} from 'typescript'; // eslint-disable-line import/no-extraneous-dependencies
import {FSHost} from './FSHost';

export class ParseConfigHost extends FSHost implements IParseConfigHost {
  filesRead: Set<FilePath>;
  useCaseSensitiveFileNames: boolean;

  // @ts-expect-error - TS2709 - Cannot use namespace 'TypeScriptModule' as a type.
  constructor(fs: FileSystem, ts: TypeScriptModule) {
    super(fs, ts);
    this.filesRead = new Set();
    this.useCaseSensitiveFileNames = ts.sys.useCaseSensitiveFileNames;
  }

  readFile(filePath: FilePath): undefined | string {
    this.filesRead.add(filePath);
    return super.readFile(filePath);
  }
}
