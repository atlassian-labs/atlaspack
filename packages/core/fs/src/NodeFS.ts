import type {ReadStream, Stats} from 'fs';
import type {Writable} from 'stream';
import type {
  FilePath,
  Encoding,
  FileOptions,
  FileSystem,
} from '@atlaspack/types-internal';
import type {
  Event,
  Options as WatcherOptions,
  AsyncSubscription,
} from '@parcel/watcher';

// @ts-expect-error - TS7016 - Could not find a declaration file for module 'graceful-fs'. '/home/ubuntu/parcel/node_modules/graceful-fs/graceful-fs.js' implicitly has an 'any' type.
import fs from 'graceful-fs';
import nativeFS from 'fs';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'ncp'. '/home/ubuntu/parcel/node_modules/ncp/lib/ncp.js' implicitly has an 'any' type.
import ncp from 'ncp';
import path from 'path';
import {tmpdir} from 'os';
import {promisify} from 'util';
import {registerSerializableClass} from '@atlaspack/core';
import {hashFile} from '@atlaspack/utils';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import watcher from '@parcel/watcher';
// @ts-expect-error - TS2732 - Cannot find module '../package.json'. Consider using '--resolveJsonModule' to import module with '.json' extension.
import packageJSON from '../package.json';

import * as searchNative from '@atlaspack/rust';
import * as searchJS from './find';

// Most of this can go away once we only support Node 10+, which includes
// require('fs').promises

const realpath = promisify(
  process.platform === 'win32' ? fs.realpath : fs.realpath.native,
);
const isPnP = process.versions.pnp != null;

function getWatchmanWatcher(): typeof watcher {
  // This is here to trick atlaspack into ignoring this require...
  const packageName = ['@atlaspack', 'watcher-watchman-js'].join('/');

  return require(packageName);
}

export class NodeFS implements FileSystem {
  readFile: any = promisify(fs.readFile);
  copyFile: any = promisify(fs.copyFile);
  stat: any = promisify(fs.stat);
  readdir: any = promisify(fs.readdir);
  symlink: any = promisify(fs.symlink);
  unlink: any = promisify(fs.unlink);
  utimes: any = promisify(fs.utimes);
  ncp: any = promisify(ncp);
  createReadStream: (path: string, options?: any) => ReadStream =
    fs.createReadStream;
  cwd: () => string = () => process.cwd();
  chdir: (directory: string) => void = (directory) => process.chdir(directory);

  statSync: (path: string) => Stats = (path) => fs.statSync(path);
  realpathSync: (path: string, cache?: any) => string =
    process.platform === 'win32' ? fs.realpathSync : fs.realpathSync.native;
  existsSync: (path: string) => boolean = fs.existsSync;
  readdirSync: any = fs.readdirSync as any;
  findAncestorFile: any = isPnP
    ? // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type. | TS2556 - A spread argument must either have a tuple type or be passed to a rest parameter.
      (...args) => searchJS.findAncestorFile(this, ...args)
    : searchNative.findAncestorFile;
  findNodeModule: any = isPnP
    ? // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type. | TS2556 - A spread argument must either have a tuple type or be passed to a rest parameter.
      (...args) => searchJS.findNodeModule(this, ...args)
    : searchNative.findNodeModule;
  findFirstFile: any = isPnP
    ? // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type. | TS2556 - A spread argument must either have a tuple type or be passed to a rest parameter.
      (...args) => searchJS.findFirstFile(this, ...args)
    : searchNative.findFirstFile;

  watcher(): typeof watcher {
    return getFeatureFlag('useWatchmanWatcher')
      ? getWatchmanWatcher()
      : watcher;
  }

  createWriteStream(filePath: string, options: any): Writable {
    // Make createWriteStream atomic
    let tmpFilePath = getTempFilePath(filePath);
    let failed = false;

    const move = async () => {
      if (!failed) {
        try {
          await fs.promises.rename(tmpFilePath, filePath);
        } catch (e: any) {
          // This is adapted from fs-write-stream-atomic. Apparently
          // Windows doesn't like renaming when the target already exists.
          if (
            process.platform === 'win32' &&
            e.syscall &&
            e.syscall === 'rename' &&
            e.code &&
            e.code === 'EPERM'
          ) {
            let [hashTmp, hashTarget] = await Promise.all([
              hashFile(this, tmpFilePath),
              hashFile(this, filePath),
            ]);

            await this.unlink(tmpFilePath);

            if (hashTmp != hashTarget) {
              throw e;
            }
          }
        }
      }
    };

    let writeStream = fs.createWriteStream(tmpFilePath, {
      ...options,
      fs: {
        ...fs,
        // @ts-expect-error - TS7006 - Parameter 'fd' implicitly has an 'any' type. | TS7006 - Parameter 'cb' implicitly has an 'any' type.
        close: (fd, cb) => {
          // @ts-expect-error - TS7006 - Parameter 'err' implicitly has an 'any' type.
          fs.close(fd, (err) => {
            if (err) {
              cb(err);
            } else {
              move().then(
                () => cb(),
                (err) => cb(err),
              );
            }
          });
        },
      },
    });

    writeStream.once('error', () => {
      failed = true;
      fs.unlinkSync(tmpFilePath);
    });

    return writeStream;
  }

  async writeFile(
    filePath: FilePath,
    contents: Buffer | string,
    options?: FileOptions | null,
  ): Promise<void> {
    let tmpFilePath = getTempFilePath(filePath);
    await fs.promises.writeFile(tmpFilePath, contents, options);
    await fs.promises.rename(tmpFilePath, filePath);
  }

  readFileSync(filePath: FilePath, encoding?: Encoding): any {
    if (encoding != null) {
      return fs.readFileSync(filePath, encoding);
    }
    return fs.readFileSync(filePath);
  }

  async realpath(originalPath: string): Promise<string> {
    try {
      return await realpath(originalPath, 'utf8');
    } catch (e: any) {
      // do nothing
    }

    return originalPath;
  }

  exists(filePath: FilePath): Promise<boolean> {
    return new Promise((resolve: (result: Promise<never>) => void) => {
      fs.exists(filePath, resolve);
    });
  }

  watch(
    dir: FilePath,
    fn: (err: Error | null | undefined, events: Array<Event>) => unknown,
    opts: WatcherOptions,
  ): Promise<AsyncSubscription> {
    return this.watcher().subscribe(dir, fn, opts);
  }

  getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>> {
    return this.watcher().getEventsSince(dir, snapshot, opts);
  }

  async writeSnapshot(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<void> {
    await this.watcher().writeSnapshot(dir, snapshot, opts);
  }

  static deserialize(): NodeFS {
    return new NodeFS();
  }

  serialize(): null {
    return null;
  }

  async mkdirp(filePath: FilePath): Promise<void> {
    await nativeFS.promises.mkdir(filePath, {recursive: true});
  }

  async rimraf(filePath: FilePath): Promise<void> {
    if (fs.promises.rm) {
      await fs.promises.rm(filePath, {recursive: true, force: true});
      return;
    }

    // fs.promises.rm is not supported in node 12...
    let stat;
    try {
      stat = await this.stat(filePath);
    } catch (err: any) {
      return;
    }

    if (stat.isDirectory()) {
      await nativeFS.promises.rmdir(filePath, {recursive: true});
    } else {
      await nativeFS.promises.unlink(filePath);
    }
  }
}

registerSerializableClass(`${packageJSON.version}:NodeFS`, NodeFS);

let writeStreamCalls = 0;

// @ts-expect-error - TS7034 - Variable 'threadId' implicitly has type 'any' in some locations where its type cannot be determined.
let threadId;
try {
  ({threadId} = require('worker_threads'));
} catch {
  //
}

// @ts-expect-error - TS7034 - Variable 'useOsTmpDir' implicitly has type 'any' in some locations where its type cannot be determined.
let useOsTmpDir;

function shouldUseOsTmpDir(filePath: FilePath) {
  // @ts-expect-error - TS7005 - Variable 'useOsTmpDir' implicitly has an 'any' type.
  if (useOsTmpDir != null) {
    // @ts-expect-error - TS7005 - Variable 'useOsTmpDir' implicitly has an 'any' type.
    return useOsTmpDir;
  }
  try {
    const tmpDir = tmpdir();
    nativeFS.accessSync(
      tmpDir,
      nativeFS.constants.R_OK | nativeFS.constants.W_OK,
    );
    const tmpDirStats = nativeFS.statSync(tmpDir);
    const filePathStats = nativeFS.statSync(filePath);
    // Check the tmpdir is on the same partition as the target directory.
    // This is required to ensure renaming is an atomic operation.
    useOsTmpDir = tmpDirStats.dev === filePathStats.dev;
  } catch (e: any) {
    // We don't have read/write access to the OS tmp directory
    useOsTmpDir = false;
  }
  return useOsTmpDir;
}

// Generate a temporary file path used for atomic writing of files.
function getTempFilePath(filePath: FilePath) {
  writeStreamCalls = writeStreamCalls % Number.MAX_SAFE_INTEGER;

  let tmpFilePath = filePath;

  // If possible, write the tmp file to the OS tmp directory
  // This reduces the amount of FS events the watcher needs to process during the build
  if (shouldUseOsTmpDir(filePath)) {
    tmpFilePath = path.join(tmpdir(), path.basename(filePath));
  }

  return (
    tmpFilePath +
    '.' +
    process.pid +
    // @ts-expect-error - TS7005 - Variable 'threadId' implicitly has an 'any' type. | TS7005 - Variable 'threadId' implicitly has an 'any' type.
    (threadId != null ? '.' + threadId : '') +
    '.' +
    (writeStreamCalls++).toString(36)
  );
}
