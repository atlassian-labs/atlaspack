import {registerSerializableClass} from '@atlaspack/core';
import packageJson from '../package.json';

let HANDLE_ID = 0;
export type HandleFunction = (...args: Array<any>) => any;

type HandleOpts = {
  fn?: HandleFunction;
  childId?: number | null | undefined;
  id?: number;
};

const handleById: Map<number, Handle> = new Map();

export default class Handle {
  id: number;
  childId: number | null | undefined;
  fn: HandleFunction | null | undefined;

  constructor(opts: HandleOpts) {
    this.id = opts.id ?? ++HANDLE_ID;
    this.fn = opts.fn;
    this.childId = opts.childId;
    handleById.set(this.id, this);
  }

  dispose() {
    handleById.delete(this.id);
  }

  serialize(): {
    childId: number | null | undefined;
    id: number;
  } {
    return {
      id: this.id,
      childId: this.childId,
    };
  }

  static deserialize(opts: HandleOpts): Handle {
    return new Handle(opts);
  }
}

// Register the Handle as a serializable class so that it will properly be deserialized
// by anything that uses WorkerFarm.
registerSerializableClass(`${packageJson.version}:Handle`, Handle);
