// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type {FileSystem, FilePath} from '@atlaspack/types-internal';
// eslint-disable-next-line flowtype/no-types-missing-file-annotation
import type {Event} from '@parcel/watcher';
import type WorkerFarm from '@atlaspack/workers';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {
  FileSystem,
  FileOptions,
  ReaddirOptions,
  Stats,
  Encoding,
  Dirent,
} from '@atlaspack/types-internal';

export const NodeFS: {
  new (): FileSystem;
};

export const MemoryFS: {
  new (farm: WorkerFarm): FileSystem;
};

export const OverlayFS: {
  new (writable: FileSystem, readable: FileSystem): FileSystem;
};

interface NodeVCSAwareFSOptions {
  gitRepoPath: null | FilePath;
  excludePatterns: Array<string>;
  logEventDiff: null | ((watcherEvents: Event[], vcsEvents: Event[]) => void);
}

export const NodeVCSAwareFS: {
  new (options: NodeVCSAwareFSOptions): FileSystem;
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SomeType = any; // replace with actual type export
