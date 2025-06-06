import {Serializable} from './worker-interface.mts';
import type {TransferItem} from './worker-interface.mts';

export class HandleRef extends Serializable {
  #ref: number;
  #transferrable: TransferItem;

  constructor(ref: number) {
    super();
    this.#ref = ref;
    this.#transferrable = this.#ref;
  }

  serialize(): TransferItem {
    return this.#transferrable;
  }

  deserialize(target: {ref: number}): HandleRef {
    return new HandleRef(target.ref);
  }
}
