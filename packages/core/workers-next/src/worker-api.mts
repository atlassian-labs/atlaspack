export class WorkerApi {
  #sharedReferences: Map<number, any>;
  #sharedReferencesByValue: Map<any, number>;

  constructor(
    sharedReferences: Map<number, any>,
    sharedReferencesByValue: Map<any, number>,
  ) {
    this.#sharedReferences = sharedReferences;
    this.#sharedReferencesByValue = sharedReferencesByValue;
  }

  getSharedReference<R = unknown>(ref: number): R | undefined {
    return this.#sharedReferences.get(ref);
  }

  resolveSharedReference(value: any): number | null {
    return this.#sharedReferencesByValue.get(value) ?? null;
  }
}
