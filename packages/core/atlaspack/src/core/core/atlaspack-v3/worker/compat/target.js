// @flow

import type {Target as NapiTarget} from '../../../../rust/index.js';
import type {
  Target as ITarget,
  FilePath,
  Environment,
  SourceLocation,
} from '../../../../types/index.js';

export class Target implements ITarget {
  distEntry: ?FilePath;
  distDir: FilePath;
  env: Environment;
  name: string;
  publicUrl: string;
  loc: ?SourceLocation;

  constructor(inner: NapiTarget, env: Environment) {
    this.distDir = inner.distDir;
    this.distEntry = inner.distEntry;
    this.name = inner.name;
    this.publicUrl = inner.publicUrl;
    this.loc = inner.loc;
    this.env = env;
  }
}
