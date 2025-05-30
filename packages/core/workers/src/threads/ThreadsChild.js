// @flow

import type {
  ChildImpl,
  MessageHandler,
  ExitHandler,
  WorkerMessage,
} from '../types';

import {isMainThread, parentPort} from 'worker_threads';

import {
  prepareForSerialization,
  restoreDeserializedObject,
} from '@atlaspack/build-cache';
import nullthrows from 'nullthrows';

import {Child} from '../child';
import {setChild} from '../childState';

export default class ThreadsChild implements ChildImpl {
  onMessage: MessageHandler;
  onExit: ExitHandler;

  constructor(onMessage: MessageHandler, onExit: ExitHandler) {
    if (isMainThread || !parentPort) {
      throw new Error('Only create ThreadsChild instances in a worker!');
    }

    this.onMessage = onMessage;
    this.onExit = onExit;
    parentPort.on('message', (data) => this.handleMessage(data));
    parentPort.on('close', this.onExit);
  }

  handleMessage(data: WorkerMessage) {
    this.onMessage(restoreDeserializedObject(data));
  }

  send(data: WorkerMessage) {
    nullthrows(parentPort).postMessage(prepareForSerialization(data));
  }
}

setChild(new Child(ThreadsChild));
