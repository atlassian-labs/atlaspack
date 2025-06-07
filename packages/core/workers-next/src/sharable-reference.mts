import {Serializable} from './worker-interface.mts';
import type {IWorker, TransferItem} from './worker-interface.mts';

export type SerializedSharableReference = number;

export class SharableReference<T> extends Serializable {
  readonly ref: number;
  #value: T;
  #sharedReferences: Map<number, any>;
  #sharedReferencesByValue: Map<any, number>;
  #workers: Array<IWorker>;

  constructor(
    ref: number,
    value: T,
    sharedReferences: Map<number, any>,
    sharedReferencesByValue: Map<any, number>,
    workers: Array<IWorker>,
  ) {
    super();
    this.ref = ref;
    this.#value = value;
    this.#sharedReferences = sharedReferences;
    this.#sharedReferencesByValue = sharedReferencesByValue;
    this.#workers = workers;
  }

  dispose = async (): Promise<void> => {
    this.#sharedReferences.delete(this.ref);
    this.#sharedReferencesByValue.delete(this.#value);
    await Promise.all(
      this.#workers.map((w) => w.deleteSharableReference(this.ref)),
    );
  };

  serialize(): TransferItem {
    return this.ref;
  }
}
