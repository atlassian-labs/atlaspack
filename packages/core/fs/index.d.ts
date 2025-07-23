import type {FileSystem, FilePath} from '@atlaspack/types-internal';
import type {Event} from '@parcel/watcher';
import type WorkerFarm from '@atlaspack/workers';

export type {
  FileSystem,
  FileOptions,
  ReaddirOptions,
  Stats,
  Encoding,
  Dirent,
} from '@atlaspack/types-internal';

export function ncp(
  sourceFS: FileSystem,
  source: FilePath,
  destinationFS: FileSystem,
  destination: FilePath,
  filter?: (filePath: FilePath) => boolean,
): Promise<void>;

export class NodeFS implements FileSystem {
  constructor();
  readFile(filePath: FilePath): Promise<Buffer>;
  readFile(filePath: FilePath, encoding: Encoding): Promise<string>;
  readFileSync(filePath: FilePath): Buffer;
  readFileSync(filePath: FilePath, encoding: Encoding): string;
  writeFile(
    filePath: FilePath,
    contents: Buffer | string,
    options?: FileOptions | null | undefined,
  ): Promise<void>;
  copyFile(
    source: FilePath,
    destination: FilePath,
    flags?: number,
  ): Promise<void>;
  stat(filePath: FilePath): Promise<Stats>;
  statSync(filePath: FilePath): Stats;
  readdir(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): Promise<FilePath[]>;
  readdir(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Promise<Dirent[]>;
  readdirSync(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): FilePath[];
  readdirSync(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Dirent[];
  symlink(target: FilePath, path: FilePath): Promise<void>;
  unlink(path: FilePath): Promise<void>;
  realpath(path: FilePath): Promise<FilePath>;
  realpathSync(path: FilePath): FilePath;
  exists(path: FilePath): Promise<boolean>;
  existsSync(path: FilePath): boolean;
  mkdirp(path: FilePath): Promise<void>;
  rimraf(path: FilePath): Promise<void>;
  ncp(source: FilePath, destination: FilePath): Promise<void>;
  createReadStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Readable;
  createWriteStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Writable;
  cwd(): FilePath;
  chdir(dir: FilePath): void;
  watch(
    dir: FilePath,
    fn: (err: Error | null | undefined, events: Array<Event>) => unknown,
    opts: WatcherOptions,
  ): Promise<AsyncSubscription>;
  getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>>;
  writeSnapshot(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<void>;
  findAncestorFile(
    fileNames: Array<string>,
    fromDir: FilePath,
    root: FilePath,
  ): FilePath | null | undefined;
  findNodeModule(
    moduleName: string,
    fromDir: FilePath,
  ): FilePath | null | undefined;
  findFirstFile(filePaths: Array<FilePath>): FilePath | null | undefined;
}

export class MemoryFS implements FileSystem {
  constructor(farm: WorkerFarm);
  readFile(filePath: FilePath): Promise<Buffer>;
  readFile(filePath: FilePath, encoding: Encoding): Promise<string>;
  readFileSync(filePath: FilePath): Buffer;
  readFileSync(filePath: FilePath, encoding: Encoding): string;
  writeFile(
    filePath: FilePath,
    contents: Buffer | string,
    options?: FileOptions | null | undefined,
  ): Promise<void>;
  copyFile(
    source: FilePath,
    destination: FilePath,
    flags?: number,
  ): Promise<void>;
  stat(filePath: FilePath): Promise<Stats>;
  statSync(filePath: FilePath): Stats;
  readdir(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): Promise<FilePath[]>;
  readdir(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Promise<Dirent[]>;
  readdirSync(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): FilePath[];
  readdirSync(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Dirent[];
  symlink(target: FilePath, path: FilePath): Promise<void>;
  unlink(path: FilePath): Promise<void>;
  realpath(path: FilePath): Promise<FilePath>;
  realpathSync(path: FilePath): FilePath;
  exists(path: FilePath): Promise<boolean>;
  existsSync(path: FilePath): boolean;
  mkdirp(path: FilePath): Promise<void>;
  rimraf(path: FilePath): Promise<void>;
  ncp(source: FilePath, destination: FilePath): Promise<void>;
  createReadStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Readable;
  createWriteStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Writable;
  cwd(): FilePath;
  chdir(dir: FilePath): void;
  watch(
    dir: FilePath,
    fn: (err: Error | null | undefined, events: Array<Event>) => unknown,
    opts: WatcherOptions,
  ): Promise<AsyncSubscription>;
  getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>>;
  writeSnapshot(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<void>;
  findAncestorFile(
    fileNames: Array<string>,
    fromDir: FilePath,
    root: FilePath,
  ): FilePath | null | undefined;
  findNodeModule(
    moduleName: string,
    fromDir: FilePath,
  ): FilePath | null | undefined;
  findFirstFile(filePaths: Array<FilePath>): FilePath | null | undefined;
}

export class OverlayFS implements FileSystem {
  constructor(writable: FileSystem, readable: FileSystem);
  readFile(filePath: FilePath): Promise<Buffer>;
  readFile(filePath: FilePath, encoding: Encoding): Promise<string>;
  readFileSync(filePath: FilePath): Buffer;
  readFileSync(filePath: FilePath, encoding: Encoding): string;
  writeFile(
    filePath: FilePath,
    contents: Buffer | string,
    options?: FileOptions | null | undefined,
  ): Promise<void>;
  copyFile(
    source: FilePath,
    destination: FilePath,
    flags?: number,
  ): Promise<void>;
  stat(filePath: FilePath): Promise<Stats>;
  statSync(filePath: FilePath): Stats;
  readdir(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): Promise<FilePath[]>;
  readdir(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Promise<Dirent[]>;
  readdirSync(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): FilePath[];
  readdirSync(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Dirent[];
  symlink(target: FilePath, path: FilePath): Promise<void>;
  unlink(path: FilePath): Promise<void>;
  realpath(path: FilePath): Promise<FilePath>;
  realpathSync(path: FilePath): FilePath;
  exists(path: FilePath): Promise<boolean>;
  existsSync(path: FilePath): boolean;
  mkdirp(path: FilePath): Promise<void>;
  rimraf(path: FilePath): Promise<void>;
  ncp(source: FilePath, destination: FilePath): Promise<void>;
  createReadStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Readable;
  createWriteStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Writable;
  cwd(): FilePath;
  chdir(dir: FilePath): void;
  watch(
    dir: FilePath,
    fn: (err: Error | null | undefined, events: Array<Event>) => unknown,
    opts: WatcherOptions,
  ): Promise<AsyncSubscription>;
  getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>>;
  writeSnapshot(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<void>;
  findAncestorFile(
    fileNames: Array<string>,
    fromDir: FilePath,
    root: FilePath,
  ): FilePath | null | undefined;
  findNodeModule(
    moduleName: string,
    fromDir: FilePath,
  ): FilePath | null | undefined;
  findFirstFile(filePaths: Array<FilePath>): FilePath | null | undefined;
}

interface NodeVCSAwareFSOptions {
  gitRepoPath: null | FilePath;
  excludePatterns: Array<string>;
  logEventDiff: null | ((watcherEvents: Event[], vcsEvents: Event[]) => void);
}

export class NodeVCSAwareFS implements FileSystem {
  constructor(options: NodeVCSAwareFSOptions);
  readFile(filePath: FilePath): Promise<Buffer>;
  readFile(filePath: FilePath, encoding: Encoding): Promise<string>;
  readFileSync(filePath: FilePath): Buffer;
  readFileSync(filePath: FilePath, encoding: Encoding): string;
  writeFile(
    filePath: FilePath,
    contents: Buffer | string,
    options?: FileOptions | null | undefined,
  ): Promise<void>;
  copyFile(
    source: FilePath,
    destination: FilePath,
    flags?: number,
  ): Promise<void>;
  stat(filePath: FilePath): Promise<Stats>;
  statSync(filePath: FilePath): Stats;
  readdir(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): Promise<FilePath[]>;
  readdir(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Promise<Dirent[]>;
  readdirSync(
    path: FilePath,
    opts?: {
      withFileTypes?: false;
    },
  ): FilePath[];
  readdirSync(
    path: FilePath,
    opts: {
      withFileTypes: true;
    },
  ): Dirent[];
  symlink(target: FilePath, path: FilePath): Promise<void>;
  unlink(path: FilePath): Promise<void>;
  realpath(path: FilePath): Promise<FilePath>;
  realpathSync(path: FilePath): FilePath;
  exists(path: FilePath): Promise<boolean>;
  existsSync(path: FilePath): boolean;
  mkdirp(path: FilePath): Promise<void>;
  rimraf(path: FilePath): Promise<void>;
  ncp(source: FilePath, destination: FilePath): Promise<void>;
  createReadStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Readable;
  createWriteStream(
    path: FilePath,
    options?: FileOptions | null | undefined,
  ): Writable;
  cwd(): FilePath;
  chdir(dir: FilePath): void;
  watch(
    dir: FilePath,
    fn: (err: Error | null | undefined, events: Array<Event>) => unknown,
    opts: WatcherOptions,
  ): Promise<AsyncSubscription>;
  getEventsSince(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<Array<Event>>;
  writeSnapshot(
    dir: FilePath,
    snapshot: FilePath,
    opts: WatcherOptions,
  ): Promise<void>;
  findAncestorFile(
    fileNames: Array<string>,
    fromDir: FilePath,
    root: FilePath,
  ): FilePath | null | undefined;
  findNodeModule(
    moduleName: string,
    fromDir: FilePath,
  ): FilePath | null | undefined;
  findFirstFile(filePaths: Array<FilePath>): FilePath | null | undefined;
  setGitRepoPath(gitRepoPath: FilePath | null): void;
}
