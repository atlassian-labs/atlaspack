import type {FilePath, Glob} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';

// @ts-expect-error TS7016
import _isGlob from 'is-glob';
// @ts-expect-error TS2305
import fastGlob, {FastGlobOptions} from 'fast-glob';
import micromatch, {isMatch, makeRe, Options} from 'micromatch';
import {normalizeSeparators} from './path';

export function isGlob(p: FilePath): any {
  return _isGlob(normalizeSeparators(p));
}

export function isGlobMatch(
  filePath: FilePath,
  glob: Glob | Array<Glob>,
  opts?: Options,
): any {
  glob = Array.isArray(glob)
    ? glob.map(normalizeSeparators)
    : normalizeSeparators(glob);
  return isMatch(filePath, glob, opts);
}

export function globMatch(
  values: Array<string>,
  glob: Glob | Array<Glob>,
  opts?: Options,
): Array<string> {
  glob = Array.isArray(glob)
    ? glob.map(normalizeSeparators)
    : normalizeSeparators(glob);

  return micromatch(values, glob, opts);
}

export function globToRegex(glob: Glob, opts?: Options): RegExp {
  return makeRe(glob, opts);
}

export function globSync(
  p: FilePath,
  fs: FileSystem,
  options?: FastGlobOptions<FilePath>,
): Array<FilePath> {
  options = {
    ...options,
    fs: {
      // @ts-expect-error TS7006
      statSync: (p) => {
        return fs.statSync(p);
      },
      // @ts-expect-error TS7006
      lstatSync: (p) => {
        // Our FileSystem interface doesn't have lstat support at the moment,
        // but this is fine for our purposes since we follow symlinks by default.
        return fs.statSync(p);
      },
      // @ts-expect-error TS7006
      readdirSync: (p, opts) => {
        return fs.readdirSync(p, opts);
      },
    },
  };

  // @ts-expect-error TS2322
  return fastGlob.sync(normalizeSeparators(p), options);
}

export function glob(
  p: FilePath,
  fs: FileSystem,
  options: FastGlobOptions<FilePath>,
): Promise<Array<FilePath>> {
  options = {
    ...options,
    fs: {
      // @ts-expect-error TS7006
      stat: async (p, cb) => {
        try {
          cb(null, await fs.stat(p));
        } catch (err: any) {
          cb(err);
        }
      },
      // @ts-expect-error TS7006
      lstat: async (p, cb) => {
        // Our FileSystem interface doesn't have lstat support at the moment,
        // but this is fine for our purposes since we follow symlinks by default.
        try {
          cb(null, await fs.stat(p));
        } catch (err: any) {
          cb(err);
        }
      },
      // @ts-expect-error TS7006
      readdir: async (p, opts, cb) => {
        if (typeof opts === 'function') {
          cb = opts;
          opts = null;
        }

        try {
          cb(null, await fs.readdir(p, opts));
        } catch (err: any) {
          cb(err);
        }
      },
    },
  };

  // @ts-expect-error TS2322
  return fastGlob(normalizeSeparators(p), options);
}
