import type {Target as NapiTarget} from '@atlaspack/rust';
import type {
  Target as ClassicTarget,
  FilePath,
  Environment,
  SourceLocation,
} from '@atlaspack/types';

export class Target implements ClassicTarget {
  distEntry: FilePath | null | undefined;
  distDir: FilePath;
  env: Environment;
  name: string;
  publicUrl: string;
  loc: SourceLocation | null | undefined;

  constructor(inner: NapiTarget, env: Environment) {
    this.distDir = inner.distDir;
    this.distEntry = inner.distEntry;
    this.name = inner.name;
    this.publicUrl = inner.publicUrl;
    this.loc = inner.loc;
    this.env = env;
  }
}
