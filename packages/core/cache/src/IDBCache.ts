import type {Cache} from './types';

// @ts-expect-error - TS2420 - Class 'IDBCache' incorrectly implements interface 'Cache'.
export class IDBCache implements Cache {
  constructor() {
    throw new Error('IDBCache is only supported in the browser');
  }
}
