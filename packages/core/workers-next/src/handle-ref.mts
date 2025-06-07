import {Serializable} from './worker-interface.mts';
import type {TransferItem} from './worker-interface.mts';

export type SerializedHandleRef = number;

export class HandleRef extends Serializable {
  #ref: number;

  constructor(ref: number) {
    super();
    this.#ref = ref;
  }

  serialize(): TransferItem {
    return this.#ref;
  }
}
