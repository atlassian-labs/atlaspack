import assert from 'assert';
import {
  filterHunkEntries,
  isHunkOnlyAssetIds,
  isHunkOnlyUnminifiedRefs,
  isHunkOnlySourceMapUrl,
  isHunkOnlySwappedVariables,
  countHunks,
} from '../src/hunk';
import type {DiffEntry} from '../src/types';

function createDiffEntry(
  type: 'equal' | 'remove' | 'add',
  line: string,
  lineNum1?: number,
  lineNum2?: number,
): DiffEntry {
  return {type, line, lineNum1, lineNum2};
}

describe('filterHunkEntries', () => {
  it('should return all entries when no filters are enabled', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'line1', 1),
      createDiffEntry('add', 'line2', undefined, 1),
    ];

    const result = filterHunkEntries(entries, false, false, false, false);

    assert.equal(result.filtered.length, 2);
    assert.equal(result.removeCount, 1);
    assert.equal(result.addCount, 1);
  });

  it('should filter entries that differ only by asset IDs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'require("a1b2c");', 1),
      createDiffEntry('add', 'require("d3e4f");', undefined, 1),
    ];

    const result = filterHunkEntries(entries, true, false, false, false);

    assert.equal(result.filtered.length, 0);
    assert.equal(result.removeCount, 0);
    assert.equal(result.addCount, 0);
  });

  it('should keep entries that differ by more than asset IDs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'require("a1b2c"); var x = 1;', 1),
      createDiffEntry('add', 'require("d3e4f"); var y = 2;', undefined, 1),
    ];

    const result = filterHunkEntries(entries, true, false, false, false);

    assert.equal(result.filtered.length, 2);
  });

  it('should filter entries that differ only by unminified refs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 1),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.foo = 1;',
        undefined,
        1,
      ),
    ];

    const result = filterHunkEntries(entries, false, true, false, false);

    assert.equal(result.filtered.length, 0);
  });

  it('should filter entries that differ only by source map URLs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '//# sourceMappingURL=file1.js.map', 1),
      createDiffEntry('add', '//# sourceMappingURL=file2.js.map', undefined, 1),
    ];

    const result = filterHunkEntries(entries, false, false, true, false);

    assert.equal(result.filtered.length, 0);
  });

  it('should filter entries that differ only by swapped variables', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'function(t){return t+1}', 1),
      createDiffEntry('add', 'function(a){return a+1}', undefined, 1),
    ];

    const result = filterHunkEntries(entries, false, false, false, true);

    assert.equal(result.filtered.length, 0);
  });

  it('should keep orphaned removes', () => {
    const entries: DiffEntry[] = [createDiffEntry('remove', 'line1', 1)];

    const result = filterHunkEntries(entries, true, true, true, true);

    assert.equal(result.filtered.length, 1);
    assert.equal(result.filtered[0].type, 'remove');
  });

  it('should keep orphaned adds', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('add', 'line1', undefined, 1),
    ];

    const result = filterHunkEntries(entries, true, true, true, true);

    assert.equal(result.filtered.length, 1);
    assert.equal(result.filtered[0].type, 'add');
  });

  it('should handle multiple filter types', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'require("a1b2c");', 1),
      createDiffEntry('add', 'require("d3e4f");', undefined, 1),
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 2),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.foo = 1;',
        undefined,
        2,
      ),
    ];

    const result = filterHunkEntries(entries, true, true, false, false);

    assert.equal(result.filtered.length, 0);
  });
});

describe('isHunkOnlyAssetIds', () => {
  it('should return true when all pairs differ only by asset IDs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'require("a1b2c");', 1),
      createDiffEntry('add', 'require("d3e4f");', undefined, 1),
    ];

    assert.equal(isHunkOnlyAssetIds(entries), true);
  });

  it('should return false when pairs differ by more than asset IDs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'require("a1b2c"); var x = 1;', 1),
      createDiffEntry('add', 'require("d3e4f"); var y = 2;', undefined, 1),
    ];

    assert.equal(isHunkOnlyAssetIds(entries), false);
  });

  it('should return false for orphaned removes', () => {
    const entries: DiffEntry[] = [createDiffEntry('remove', 'line1', 1)];

    assert.equal(isHunkOnlyAssetIds(entries), false);
  });

  it('should return false for orphaned adds', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('add', 'line1', undefined, 1),
    ];

    assert.equal(isHunkOnlyAssetIds(entries), false);
  });

  it('should return true for multiple pairs with only asset ID differences', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'require("a1b2c");', 1),
      createDiffEntry('add', 'require("d3e4f");', undefined, 1),
      createDiffEntry('remove', 'var $x1y2z = 123;', 2),
      createDiffEntry('add', 'var $a3b4c = 123;', undefined, 2),
    ];

    assert.equal(isHunkOnlyAssetIds(entries), true);
  });
});

describe('isHunkOnlyUnminifiedRefs', () => {
  it('should return true when all pairs differ only by unminified refs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 1),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.foo = 1;',
        undefined,
        1,
      ),
    ];

    assert.equal(isHunkOnlyUnminifiedRefs(entries), true);
  });

  it('should return false when pairs differ by more than unminified refs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 1),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.bar = 1;',
        undefined,
        1,
      ),
    ];

    assert.equal(isHunkOnlyUnminifiedRefs(entries), false);
  });

  it('should return false when remove/add counts differ', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 1),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.foo = 1;',
        undefined,
        1,
      ),
      createDiffEntry(
        'add',
        '$11111ef5514babaf$exports.bar = 2;',
        undefined,
        2,
      ),
    ];

    assert.equal(isHunkOnlyUnminifiedRefs(entries), false);
  });

  it('should return false for lines without unminified ref patterns', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'var a = 1;', 1),
      createDiffEntry('add', 'var b = 1;', undefined, 1),
    ];

    assert.equal(isHunkOnlyUnminifiedRefs(entries), false);
  });
});

describe('isHunkOnlySourceMapUrl', () => {
  it('should return true when all pairs differ only by source map URLs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '//# sourceMappingURL=file1.js.map', 1),
      createDiffEntry('add', '//# sourceMappingURL=file2.js.map', undefined, 1),
    ];

    assert.equal(isHunkOnlySourceMapUrl(entries), true);
  });

  it('should return false when pairs differ by more than source map URLs', () => {
    const entries: DiffEntry[] = [
      createDiffEntry(
        'remove',
        'var a = 1; //# sourceMappingURL=file1.js.map',
        1,
      ),
      createDiffEntry(
        'add',
        'var b = 1; //# sourceMappingURL=file2.js.map',
        undefined,
        1,
      ),
    ];

    assert.equal(isHunkOnlySourceMapUrl(entries), false);
  });

  it('should return false for orphaned entries', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', '//# sourceMappingURL=file1.js.map', 1),
    ];

    assert.equal(isHunkOnlySourceMapUrl(entries), false);
  });
});

describe('isHunkOnlySwappedVariables', () => {
  it('should return true when all pairs differ only by swapped variables', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'function(t){return t+1}', 1),
      createDiffEntry('add', 'function(a){return a+1}', undefined, 1),
    ];

    assert.equal(isHunkOnlySwappedVariables(entries), true);
  });

  it('should return false when pairs differ by more than swapped variables', () => {
    const entries: DiffEntry[] = [
      createDiffEntry('remove', 'var t = a + b;', 1),
      createDiffEntry('add', 'var a = t + c;', undefined, 1),
    ];

    assert.equal(isHunkOnlySwappedVariables(entries), false);
  });

  it('should return false for orphaned entries', () => {
    const entries: DiffEntry[] = [createDiffEntry('remove', 'var t = a;', 1)];

    assert.equal(isHunkOnlySwappedVariables(entries), false);
  });
});

describe('countHunks', () => {
  it('should count hunks in simple diff', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'line2', 2),
      createDiffEntry('add', 'line2modified', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const count = countHunks(diff);

    assert.equal(count, 1);
  });

  it('should count multiple hunks', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('remove', 'line1', 1),
      createDiffEntry('add', 'line1modified', undefined, 1),
      createDiffEntry('equal', 'line2', 2, 2),
      createDiffEntry('remove', 'line3', 3),
      createDiffEntry('add', 'line3modified', undefined, 3),
    ];

    const count = countHunks(diff);

    assert.equal(count, 2);
  });

  it('should return 0 for identical files', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('equal', 'line2', 2, 2),
    ];

    const count = countHunks(diff);

    assert.equal(count, 0);
  });

  it('should filter hunks with only asset ID differences', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'require("a1b2c");', 2),
      createDiffEntry('add', 'require("d3e4f");', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const count = countHunks(diff, true, false, false, false);

    assert.equal(count, 0);
  });

  it('should filter hunks with only unminified ref differences', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 2),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.foo = 1;',
        undefined,
        2,
      ),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const count = countHunks(diff, false, true, false, false);

    assert.equal(count, 0);
  });

  it('should filter hunks with only source map URL differences', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', '//# sourceMappingURL=file1.js.map', 2),
      createDiffEntry('add', '//# sourceMappingURL=file2.js.map', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const count = countHunks(diff, false, false, true, false);

    assert.equal(count, 0);
  });

  it('should filter hunks with only swapped variable differences', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'function(t){return t+1}', 2),
      createDiffEntry('add', 'function(a){return a+1}', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const count = countHunks(diff, false, false, false, true);

    assert.equal(count, 0);
  });

  it('should count hunk at end of diff', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'line2', 2),
      createDiffEntry('add', 'line2modified', undefined, 2),
    ];

    const count = countHunks(diff);

    assert.equal(count, 1);
  });

  it('should handle multiple filter types', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'require("a1b2c");', 2),
      createDiffEntry('add', 'require("d3e4f");', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
      createDiffEntry('remove', '$e3f4b1abd74dab96$exports.foo = 1;', 4),
      createDiffEntry(
        'add',
        '$00042ef5514babaf$exports.foo = 1;',
        undefined,
        4,
      ),
    ];

    const count = countHunks(diff, true, true, false, false);

    assert.equal(count, 0);
  });

  it('should not filter hunks with real differences even when filters enabled', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'require("a1b2c"); var x = 1;', 2),
      createDiffEntry('add', 'require("d3e4f"); var y = 2;', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const count = countHunks(diff, true, true, true, true);

    assert.equal(count, 1);
  });
});
