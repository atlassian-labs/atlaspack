import assert from 'assert';
import {printDiff} from '../src/print';
import type {DiffEntry} from '../src/types';

function createDiffEntry(
  type: 'equal' | 'remove' | 'add',
  line: string,
  lineNum1?: number,
  lineNum2?: number,
): DiffEntry {
  return {type, line, lineNum1, lineNum2};
}

describe('printDiff', () => {
  let consoleOutput: string[];
  let originalLog: typeof console.log;

  beforeEach(() => {
    consoleOutput = [];
    // eslint-disable-next-line no-console
    originalLog = console.log;
    // eslint-disable-next-line no-console
    console.log = (...args: any[]) => {
      consoleOutput.push(args.join(' '));
    };
  });

  afterEach(() => {
    // eslint-disable-next-line no-console
    console.log = originalLog;
  });

  it('should print identical files message when no changes', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('equal', 'line2', 2, 2),
    ];

    const result = printDiff(diff, 'file1.js', 'file2.js');

    assert.equal(result.hunkCount, 0);
    assert.equal(result.hasChanges, false);
    assert(consoleOutput.some((line) => line.includes('Files are identical')));
  });

  it('should print diff for simple changes', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'line2', 2),
      createDiffEntry('add', 'line2modified', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const result = printDiff(diff, 'file1.js', 'file2.js');

    assert.equal(result.hunkCount, 1);
    assert.equal(result.hasChanges, true);
    assert(consoleOutput.some((line) => line.includes('Comparing files')));
    assert(consoleOutput.some((line) => line.includes('file1.js')));
    assert(consoleOutput.some((line) => line.includes('file2.js')));
  });

  it('should include context lines', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'context1', 1, 1),
      createDiffEntry('equal', 'context2', 2, 2),
      createDiffEntry('equal', 'context3', 3, 3),
      createDiffEntry('remove', 'line2', 4),
      createDiffEntry('add', 'line2modified', undefined, 4),
      createDiffEntry('equal', 'context4', 5, 5),
      createDiffEntry('equal', 'context5', 6, 6),
      createDiffEntry('equal', 'context6', 7, 7),
    ];

    printDiff(diff, 'file1.js', 'file2.js', 3);

    // Should include context before and after changes
    const output = consoleOutput.join('\n');
    assert(output.includes('context3')); // Context before
    assert(output.includes('context4')); // Context after
  });

  it('should filter hunks with only asset ID differences', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'require("a1b2c");', 2),
      createDiffEntry('add', 'require("d3e4f");', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const result = printDiff(
      diff,
      'file1.js',
      'file2.js',
      3,
      true, // ignoreAssetIds
      false,
      false,
      false,
    );

    assert.equal(result.hunkCount, 0);
    assert(
      consoleOutput.some((line) => line.includes('No significant changes')),
    );
  });

  it('should print hunk headers', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('remove', 'line1', 1),
      createDiffEntry('add', 'line1modified', undefined, 1),
      createDiffEntry('equal', 'line2', 2, 2),
    ];

    printDiff(diff, 'file1.js', 'file2.js');

    const output = consoleOutput.join('\n');
    assert(output.includes('@@')); // Hunk header format
  });

  it('should handle summary mode', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('remove', 'line1', 1),
      createDiffEntry('add', 'line1modified', undefined, 1),
      createDiffEntry('equal', 'line2', 2, 2),
    ];

    const result = printDiff(
      diff,
      'file1.js',
      'file2.js',
      3,
      false,
      false,
      false,
      false,
      true, // summaryMode
    );

    assert.equal(result.hunkCount, 1);
    assert.equal(result.hasChanges, true);
    // In summary mode, should not print full diff
    assert(!consoleOutput.some((line) => line.includes('-')));
    assert(!consoleOutput.some((line) => line.includes('+')));
  });

  it('should filter multiple ignore types', () => {
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

    const result = printDiff(
      diff,
      'file1.js',
      'file2.js',
      3,
      true, // ignoreAssetIds
      true, // ignoreUnminifiedRefs
      false,
      false,
    );

    assert.equal(result.hunkCount, 0);
  });

  it('should print changes when filters do not match', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'require("a1b2c"); var x = 1;', 2),
      createDiffEntry('add', 'require("d3e4f"); var y = 2;', undefined, 2),
      createDiffEntry('equal', 'line3', 3, 3),
    ];

    const result = printDiff(
      diff,
      'file1.js',
      'file2.js',
      3,
      true, // ignoreAssetIds (but there are other differences)
      false,
      false,
      false,
    );

    assert.equal(result.hunkCount, 1);
    assert.equal(result.hasChanges, true);
  });

  it('should handle hunk at end of diff', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('equal', 'line1', 1, 1),
      createDiffEntry('remove', 'line2', 2),
      createDiffEntry('add', 'line2modified', undefined, 2),
    ];

    const result = printDiff(diff, 'file1.js', 'file2.js');

    assert.equal(result.hunkCount, 1);
    assert.equal(result.hasChanges, true);
  });

  it('should handle multiple hunks', () => {
    const diff: DiffEntry[] = [
      createDiffEntry('remove', 'line1', 1),
      createDiffEntry('add', 'line1modified', undefined, 1),
      createDiffEntry('equal', 'line2', 2, 2),
      createDiffEntry('remove', 'line3', 3),
      createDiffEntry('add', 'line3modified', undefined, 3),
    ];

    const result = printDiff(diff, 'file1.js', 'file2.js');

    assert.equal(result.hunkCount, 2);
    assert.equal(result.hasChanges, true);
  });
});
