// @flow strict-local

import {cache} from './setup-cache';

const mochaHooks = {
  async beforeEach(): Promise<void> {
    const keys = await cache.keys();
    for (const key of keys) {
      await cache.store.delete(key);
    }
  },
};

export {mochaHooks};
