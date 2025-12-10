import assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {readAndDeminify} from '../../src/utils/deminify';

describe('readAndDeminify', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dist-differ-test-'));
  });

  afterEach(() => {
    fs.rmSync(tempDir, {recursive: true, force: true});
  });

  it('should read and split file on semicolons', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, 'var a=1;var b=2;var c=3;');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 3);
    assert.equal(result[0], 'var a=1;');
    assert.equal(result[1], 'var b=2;');
    assert.equal(result[2], 'var c=3;');
  });

  it('should read and split file on commas', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, 'var a=1,var b=2,var c=3,');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 3);
    assert.equal(result[0], 'var a=1,');
    assert.equal(result[1], 'var b=2,');
    assert.equal(result[2], 'var c=3,');
  });

  it('should handle mixed semicolons and commas', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, 'var a=1;var b=2,var c=3;');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 3);
    assert.equal(result[0], 'var a=1;');
    assert.equal(result[1], 'var b=2,');
    assert.equal(result[2], 'var c=3;');
  });

  it('should handle empty file', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, '');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 0);
  });

  it('should handle file with no delimiters', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, 'var a = 1');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 1);
    assert.equal(result[0], 'var a = 1');
  });

  it('should handle trailing content after last delimiter', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, 'var a=1;var b=2;var c=3');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 3);
    assert.equal(result[0], 'var a=1;');
    assert.equal(result[1], 'var b=2;');
    assert.equal(result[2], 'var c=3');
  });

  it('should handle file with only whitespace', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, '   \n\t  ');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 0);
  });

  it('should handle file with multiple consecutive delimiters', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(filePath, 'var a=1;;var b=2,,');

    const result = readAndDeminify(filePath);

    assert(result !== null);
    // Each delimiter creates a new line, so ;; creates two lines, ,, creates two lines
    assert(result.length >= 2);
    assert(result.some((line) => line.includes('var a=1')));
    assert(result.some((line) => line.includes('var b=2')));
  });

  it('should return null and set exit code for non-existent file', () => {
    const originalExitCode = process.exitCode;
    const filePath = path.join(tempDir, 'nonexistent.js');

    const result = readAndDeminify(filePath);

    assert.equal(result, null);
    assert.equal(process.exitCode, 1);
    process.exitCode = originalExitCode;
  });

  it('should handle minified code with no spaces', () => {
    const filePath = path.join(tempDir, 'test.js');
    fs.writeFileSync(
      filePath,
      'function a(){return"test"}function b(){return"test2"}',
    );

    const result = readAndDeminify(filePath);

    assert(result !== null);
    assert.equal(result.length, 1); // No delimiters, so it's all one line
    assert.equal(
      result[0],
      'function a(){return"test"}function b(){return"test2"}',
    );
  });
});
