import type {Cache} from './types';

export class IDBCache implements Cache {
  constructor() {
    throw new Error('IDBCache is only supported in the browser');
  }
}
