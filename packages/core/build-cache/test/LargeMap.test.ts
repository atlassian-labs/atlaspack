import assert from 'assert';
import {LargeMap} from '../src/LargeMap';

describe('LargeMap', () => {
  describe('basic operations', () => {
    it('should set and get values', () => {
      const map = new LargeMap<string, number>();
      map.set('a', 1);
      map.set('b', 2);

      assert.equal(map.get('a'), 1);
      assert.equal(map.get('b'), 2);
    });

    it('should return undefined for missing keys', () => {
      const map = new LargeMap<string, number>();
      assert.equal(map.get('missing'), undefined);
    });

    it('should check if key exists with has()', () => {
      const map = new LargeMap<string, number>();
      map.set('a', 1);

      assert.equal(map.has('a'), true);
      assert.equal(map.has('b'), false);
    });

    it('should update existing key in single map', () => {
      const map = new LargeMap<string, number>();
      map.set('a', 1);
      map.set('a', 2);

      assert.equal(map.get('a'), 2);
    });

    it('should return this from set() for chaining', () => {
      const map = new LargeMap<string, number>();
      const result = map.set('a', 1).set('b', 2).set('c', 3);

      assert.equal(result, map);
      assert.equal(map.get('a'), 1);
      assert.equal(map.get('b'), 2);
      assert.equal(map.get('c'), 3);
    });
  });

  describe('multiple internal maps', () => {
    it('should create new internal map when maxSize is reached', () => {
      const map = new LargeMap<number, number>(3); // Small maxSize for testing

      map.set(1, 1);
      map.set(2, 2);
      map.set(3, 3);
      assert.equal(map.maps.length, 1);

      map.set(4, 4); // Should trigger new internal map
      assert.equal(map.maps.length, 2);

      map.set(5, 5);
      map.set(6, 6);
      assert.equal(map.maps.length, 2);

      map.set(7, 7); // Should trigger another new internal map
      assert.equal(map.maps.length, 3);
    });

    it('should find values across multiple internal maps', () => {
      const map = new LargeMap<number, string>(2);

      map.set(1, 'one');
      map.set(2, 'two');
      map.set(3, 'three');
      map.set(4, 'four');
      map.set(5, 'five');

      assert.equal(map.get(1), 'one');
      assert.equal(map.get(2), 'two');
      assert.equal(map.get(3), 'three');
      assert.equal(map.get(4), 'four');
      assert.equal(map.get(5), 'five');
    });

    it('should check has() across multiple internal maps', () => {
      const map = new LargeMap<number, string>(2);

      map.set(1, 'one');
      map.set(2, 'two');
      map.set(3, 'three');

      assert.equal(map.has(1), true);
      assert.equal(map.has(2), true);
      assert.equal(map.has(3), true);
      assert.equal(map.has(4), false);
    });

    it('should update existing key in earlier internal map', () => {
      const map = new LargeMap<number, string>(2);

      map.set(1, 'one');
      map.set(2, 'two');
      map.set(3, 'three'); // Goes to second map

      // Update key in first map
      map.set(1, 'ONE');

      assert.equal(map.get(1), 'ONE');
      assert.equal(map.maps.length, 2); // Should not create new map
    });
  });

  describe('type handling', () => {
    it('should work with object keys', () => {
      const map = new LargeMap<object, string>();
      const key1 = {id: 1};
      const key2 = {id: 2};

      map.set(key1, 'first');
      map.set(key2, 'second');

      assert.equal(map.get(key1), 'first');
      assert.equal(map.get(key2), 'second');
      assert.equal(map.get({id: 1}), undefined); // Different reference
    });

    it('should work with null and undefined values', () => {
      const map = new LargeMap<string, any>();

      map.set('null', null);
      map.set('undefined', undefined);

      assert.equal(map.get('null'), null);
      assert.equal(map.get('undefined'), undefined);
      assert.equal(map.has('null'), true);
      assert.equal(map.has('undefined'), true);
    });

    it('should work with various key types', () => {
      const map = new LargeMap<any, string>();

      map.set(1, 'number');
      map.set('str', 'string');
      map.set(true, 'boolean');
      map.set(null, 'null');

      assert.equal(map.get(1), 'number');
      assert.equal(map.get('str'), 'string');
      assert.equal(map.get(true), 'boolean');
      assert.equal(map.get(null), 'null');
    });
  });
});
