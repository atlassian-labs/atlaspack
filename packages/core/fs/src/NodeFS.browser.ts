import type {FileSystem} from '@atlaspack/types-internal';

export class NodeFS implements FileSystem {
  constructor() {
    throw new Error("NodeFS isn't available in the browser");
  }
}
