import type {FileSystem} from '@atlaspack/types-internal';

// @ts-expect-error TS2420
export class NodeFS implements FileSystem {
  constructor() {
    throw new Error("NodeFS isn't available in the browser");
  }
}
