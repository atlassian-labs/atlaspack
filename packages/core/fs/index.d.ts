import type {FileSystem} from '@atlaspack/types-internal';
import type WorkerFarm from '@atlaspack/workers';
import type {NodeVCSAwareFSOptions} from './src/NodeVCSAwareFS';

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
