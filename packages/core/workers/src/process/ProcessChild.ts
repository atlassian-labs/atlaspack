import type {
  ChildImpl,
  MessageHandler,
  ExitHandler,
  WorkerMessage,
} from '../types';

import {serialize, deserialize} from '@atlaspack/build-cache';
import nullthrows from 'nullthrows';

import {Child} from '../child';
import {setChild} from '../childState';

// @ts-expect-error TS2420
export default class ProcessChild implements ChildImpl {
  onMessage: MessageHandler;
  onExit: ExitHandler;

  constructor(onMessage: MessageHandler, onExit: ExitHandler) {
    if (!process.send) {
      throw new Error('Only create ProcessChild instances in a worker!');
    }

    this.onMessage = onMessage;
    this.onExit = onExit;
    // @ts-expect-error TS2345
    process.on('message', (data) => this.handleMessage(data));
  }

  handleMessage(data: string): void {
    if (data === 'die') {
      return this.stop();
    }

    this.onMessage(deserialize(Buffer.from(data, 'base64')));
  }

  send(data: WorkerMessage) {
    let processSend = nullthrows(process.send).bind(process);
    processSend(serialize(data).toString('base64'), (err: any) => {
      if (err && err instanceof Error) {
        // @ts-expect-error TS2339
        if (err.code === 'ERR_IPC_CHANNEL_CLOSED') {
          // IPC connection closed
          // no need to keep the worker running if it can't send or receive data
          return this.stop();
        }
      }
    });
  }

  stop() {
    this.onExit(0);
    process.exit();
  }
}

// @ts-expect-error TS2345
setChild(new Child(ProcessChild));
