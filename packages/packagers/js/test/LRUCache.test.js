// @flow strict-local

import {LRUCache} from '../src/LRUCache';
import assert from 'assert';

describe('LRUCache', () => {
  it('should cache values', () => {
    const cache = new LRUCache(10);

    cache.set('a', 1);
    cache.set('b', 2);
    cache.set('c', 3);

    assert.equal(cache.get('a'), 1);
    assert.equal(cache.getLRU(), 'a');
    assert.equal(cache.get('b'), 2);
    assert.equal(cache.getLRU(), 'b');
    assert.equal(cache.get('c'), 3);
    assert.equal(cache.getLRU(), 'c');
  });

  it('should evict least recently used items', () => {
    const cache = new LRUCache(2);

    cache.set('a', 1);
    cache.set('b', 2);
    cache.set('c', 3);

    assert.equal(cache.get('a'), null);
    // implementation decision here that `get` on missing entries will not update the LRU list
    // on practical usages, a set will be called after this case so it does not matter
    assert.equal(cache.getLRU(), 'c');
    assert.equal(cache.get('b'), 2);
    assert.equal(cache.getLRU(), 'b');
    assert.equal(cache.get('c'), 3);
  });

  it('should evict least recently used items', () => {
    const cache = new LRUCache(2);

    cache.set('a', 1);
    cache.set('b', 2);
    cache.get('a');
    cache.set('c', 3);

    assert.equal(cache.get('a'), 1);
    assert.equal(cache.get('b'), null);
    assert.equal(cache.get('c'), 3);
  });
});
