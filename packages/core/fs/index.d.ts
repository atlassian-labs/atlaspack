import type {FileSystem, FilePath} from '@atlaspack/types-internal';
import type WorkerFarm from '@atlaspack/workers';

export interface NodeVCSAwareFSOptions {
  gitRepoPath: FilePath;
  excludePatterns: Array<string>;
  logEventDiff: (watcherEvents: Event[], vcsEvents: Event[]) => void;
}

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

export const NodeVCSAwareFS: {
  new (options: NodeVCSAwareFSOptions): FileSystem;
};
