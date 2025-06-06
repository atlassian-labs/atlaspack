import {Serializable} from './worker-interface.mts';
import type {Transferrable} from './worker-interface.mts';

export class HandleRef extends Serializable {
  #ref: number
  #transferrable: Transferrable

  constructor(ref: number) {
    super()
    this.#ref = ref
    this.#transferrable = this.#ref
  }

  serialize(): Transferrable {
    return this.#transferrable
  }

  deserialize(target: { ref: number }): HandleRef {
    return new HandleRef(target.ref)
  }
}
