/* eslint-env worker*/

import type {
  ChildImpl,
  MessageHandler,
  ExitHandler,
  WorkerMessage,
} from '../types';
import {setChild} from '../childState';
import {Child} from '../child';
import {
  prepareForSerialization,
  restoreDeserializedObject,
} from '@atlaspack/core';

// @ts-expect-error - TS2420 - Class 'WebChild' incorrectly implements interface 'ChildImpl'.
export default class WebChild implements ChildImpl {
  onMessage: MessageHandler;
  onExit: ExitHandler;

  constructor(onMessage: MessageHandler, onExit: ExitHandler) {
    if (
      !(
        typeof WorkerGlobalScope !== 'undefined' &&
        self instanceof WorkerGlobalScope
      )
    ) {
      throw new Error('Only create WebChild instances in a worker!');
    }

    this.onMessage = onMessage;
    this.onExit = onExit;
    self.addEventListener('message', ({data}: MessageEvent) => {
      if (data === 'stop') {
        this.onExit(0);
        self.postMessage('stopped');
      }
      this.handleMessage(data);
    });
    self.postMessage('online');
  }

  handleMessage(data: WorkerMessage) {
    this.onMessage(restoreDeserializedObject(data));
  }

  send(data: WorkerMessage) {
    self.postMessage(prepareForSerialization(data));
  }
}

setChild(new Child(WebChild));
