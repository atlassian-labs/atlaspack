import assert from 'assert';
import {computeDiff} from '../src/diff';
import type {DiffEntry} from '../src/types';

describe('computeDiff', () => {
  it('should return empty diff for identical arrays', () => {
    const lines = ['line1', 'line2', 'line3'];
    const diff = computeDiff(lines, lines);

    assert.equal(diff.length, 3);
    assert.equal(diff[0].type, 'equal');
    assert.equal(diff[1].type, 'equal');
    assert.equal(diff[2].type, 'equal');
    assert.equal(diff[0].line, 'line1');
    assert.equal(diff[0].lineNum1, 1);
    assert.equal(diff[0].lineNum2, 1);
  });

  it('should detect additions only', () => {
    const lines1: string[] = [];
    const lines2 = ['line1', 'line2'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 2);
    assert.equal(diff[0].type, 'add');
    assert.equal(diff[0].line, 'line1');
    assert.equal(diff[0].lineNum2, 1);
    assert.equal(diff[1].type, 'add');
    assert.equal(diff[1].line, 'line2');
    assert.equal(diff[1].lineNum2, 2);
  });

  it('should detect deletions only', () => {
    const lines1 = ['line1', 'line2'];
    const lines2: string[] = [];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 2);
    assert.equal(diff[0].type, 'remove');
    assert.equal(diff[0].line, 'line1');
    assert.equal(diff[0].lineNum1, 1);
    assert.equal(diff[1].type, 'remove');
    assert.equal(diff[1].line, 'line2');
    assert.equal(diff[1].lineNum1, 2);
  });

  it('should detect modifications', () => {
    const lines1 = ['line1', 'line2', 'line3'];
    const lines2 = ['line1', 'line2modified', 'line3'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 4);
    assert.equal(diff[0].type, 'equal');
    assert.equal(diff[0].line, 'line1');
    assert.equal(diff[1].type, 'remove');
    assert.equal(diff[1].line, 'line2');
    assert.equal(diff[1].lineNum1, 2);
    assert.equal(diff[2].type, 'add');
    assert.equal(diff[2].line, 'line2modified');
    assert.equal(diff[2].lineNum2, 2);
    assert.equal(diff[3].type, 'equal');
    assert.equal(diff[3].line, 'line3');
  });

  it('should handle different length files', () => {
    const lines1 = ['line1', 'line2'];
    const lines2 = ['line1', 'line2', 'line3', 'line4'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 4);
    assert.equal(diff[0].type, 'equal');
    assert.equal(diff[1].type, 'equal');
    assert.equal(diff[2].type, 'add');
    assert.equal(diff[2].line, 'line3');
    assert.equal(diff[3].type, 'add');
    assert.equal(diff[3].line, 'line4');
  });

  it('should handle first file longer than second', () => {
    const lines1 = ['line1', 'line2', 'line3', 'line4'];
    const lines2 = ['line1', 'line2'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 4);
    assert.equal(diff[0].type, 'equal');
    assert.equal(diff[1].type, 'equal');
    assert.equal(diff[2].type, 'remove');
    assert.equal(diff[2].line, 'line3');
    assert.equal(diff[3].type, 'remove');
    assert.equal(diff[3].line, 'line4');
  });

  it('should handle multiple modifications', () => {
    const lines1 = ['line1', 'line2', 'line3', 'line4'];
    const lines2 = ['line1modified', 'line2', 'line3modified', 'line4'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 6);
    assert.equal(diff[0].type, 'remove');
    assert.equal(diff[0].line, 'line1');
    assert.equal(diff[1].type, 'add');
    assert.equal(diff[1].line, 'line1modified');
    assert.equal(diff[2].type, 'equal');
    assert.equal(diff[2].line, 'line2');
    assert.equal(diff[3].type, 'remove');
    assert.equal(diff[3].line, 'line3');
    assert.equal(diff[4].type, 'add');
    assert.equal(diff[4].line, 'line3modified');
    assert.equal(diff[5].type, 'equal');
    assert.equal(diff[5].line, 'line4');
  });

  it('should handle empty arrays', () => {
    const diff = computeDiff([], []);

    assert.equal(diff.length, 0);
  });

  it('should handle single line differences', () => {
    const lines1 = ['line1'];
    const lines2 = ['line2'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 2);
    assert.equal(diff[0].type, 'remove');
    assert.equal(diff[0].line, 'line1');
    assert.equal(diff[1].type, 'add');
    assert.equal(diff[1].line, 'line2');
  });

  it('should correctly set line numbers', () => {
    const lines1 = ['a', 'b', 'c'];
    const lines2 = ['a', 'x', 'c'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff[0].lineNum1, 1);
    assert.equal(diff[0].lineNum2, 1);
    assert.equal(diff[1].lineNum1, 2);
    assert.equal(diff[2].lineNum2, 2);
    assert.equal(diff[3].lineNum1, 3);
    assert.equal(diff[3].lineNum2, 3);
  });

  it('should handle all lines different', () => {
    const lines1 = ['a', 'b', 'c'];
    const lines2 = ['x', 'y', 'z'];

    const diff = computeDiff(lines1, lines2);

    assert.equal(diff.length, 6);
    for (let i = 0; i < 3; i++) {
      assert.equal(diff[i * 2].type, 'remove');
      assert.equal(diff[i * 2 + 1].type, 'add');
    }
  });
});
