// @flow

import type {FilePath} from '@atlaspack/types-internal';
import type {
  CallRequest,
  HandleCallRequest,
  WorkerRequest,
  SharedReference,
} from './types';
import type {HandleFunction} from './Handle';

import nullthrows from 'nullthrows';
import {deserialize, serialize} from '@atlaspack/build-cache';
import Worker from './Worker';
import Handle from './Handle';

export class WorkerApi {
  #workers: Map<number, Worker>;
  #sharedReferences: Map<SharedReference, mixed>;
  #sharedReferencesByValue: Map<mixed, SharedReference>;
  #processRequest: (
    data: {|location: FilePath|} & $Shape<WorkerRequest>,
    worker?: Worker,
  ) => Promise<?string>;

  constructor(
    workers: Map<number, Worker>,
    sharedReferences: Map<SharedReference, mixed>,
    sharedReferencesByValue: Map<mixed, SharedReference>,
    processRequest: (
      data: {|location: FilePath|} & $Shape<WorkerRequest>,
      worker?: Worker,
    ) => Promise<?string>,
  ) {
    this.#workers = workers;
    this.#sharedReferences = sharedReferences;
    this.#sharedReferencesByValue = sharedReferencesByValue;
    this.#processRequest = processRequest;
  }

  async callMaster(
    request: CallRequest,
    awaitResponse: ?boolean = true,
  ): Promise<mixed> {
    // $FlowFixMe
    let result = await this.#processRequest({
      ...request,
      awaitResponse,
    });
    return deserialize(serialize(result));
  }

  createReverseHandle(fn: HandleFunction): Handle {
    return this.createReverseHandle(fn);
  }

  callChild(childId: number, request: HandleCallRequest): Promise<mixed> {
    return new Promise((resolve, reject) => {
      nullthrows(this.#workers.get(childId)).call({
        ...request,
        resolve,
        reject,
        retries: 0,
      });
    });
  }

  runHandle(handle: Handle, args: Array<any>): Promise<mixed> {
    return this.callChild(nullthrows(handle.childId), {
      handle: handle.id,
      args,
    });
  }

  getSharedReference(ref: SharedReference): mixed {
    return this.#sharedReferences.get(ref);
  }

  resolveSharedReference(value: mixed): SharedReference | void {
    return this.#sharedReferencesByValue.get(value);
  }
}
