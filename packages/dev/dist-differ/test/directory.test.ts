import assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {getAllFiles, compareDirectories} from '../src/directory';
import {createContext} from '../src/context';

describe('getAllFiles', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dist-differ-test-'));
  });

  afterEach(() => {
    fs.rmSync(tempDir, {recursive: true, force: true});
  });

  it('should get all .js files from directory', () => {
    fs.writeFileSync(path.join(tempDir, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(tempDir, 'file2.js'), 'content2');
    fs.writeFileSync(path.join(tempDir, 'file3.txt'), 'content3'); // Should be ignored

    const files = getAllFiles(tempDir);

    assert.equal(files.length, 2);
    assert(files.some((f) => f.filename === 'file1.js'));
    assert(files.some((f) => f.filename === 'file2.js'));
    assert(!files.some((f) => f.filename === 'file3.txt'));
  });

  it('should get files recursively from subdirectories', () => {
    const subDir = path.join(tempDir, 'subdir');
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(tempDir, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(subDir, 'file2.js'), 'content2');

    const files = getAllFiles(tempDir);

    assert.equal(files.length, 2);
    assert(files.some((f) => f.filename === 'file1.js'));
    assert(files.some((f) => f.filename === 'file2.js'));
  });

  it('should set correct relative paths', () => {
    const subDir = path.join(tempDir, 'subdir');
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(tempDir, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(subDir, 'file2.js'), 'content2');

    const files = getAllFiles(tempDir);

    const file1 = files.find((f) => f.filename === 'file1.js');
    const file2 = files.find((f) => f.filename === 'file2.js');

    assert(file1);
    assert.equal(file1.relativePath, 'file1.js');
    assert(file2);
    assert.equal(file2.relativePath, 'subdir/file2.js');
  });

  it('should include file size', () => {
    const content = 'test content';
    fs.writeFileSync(path.join(tempDir, 'file1.js'), content);

    const files = getAllFiles(tempDir);

    assert.equal(files.length, 1);
    assert.equal(files[0].size, content.length);
  });

  it('should return empty array for empty directory', () => {
    const files = getAllFiles(tempDir);

    assert.equal(files.length, 0);
  });

  it('should ignore .map files', () => {
    fs.writeFileSync(path.join(tempDir, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(tempDir, 'file1.js.map'), 'content2');

    const files = getAllFiles(tempDir);

    assert.equal(files.length, 1);
    assert.equal(files[0].filename, 'file1.js');
  });
});

describe('compareDirectories', () => {
  let tempDir: string;
  let dir1: string;
  let dir2: string;
  let consoleOutput: string[];
  let originalLog: typeof console.log;
  let originalExitCode: number | undefined;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dist-differ-test-'));
    dir1 = path.join(tempDir, 'dir1');
    dir2 = path.join(tempDir, 'dir2');
    fs.mkdirSync(dir1);
    fs.mkdirSync(dir2);

    consoleOutput = [];
    // eslint-disable-next-line no-console
    originalLog = console.log;
    // eslint-disable-next-line no-console
    console.log = (...args: any[]) => {
      consoleOutput.push(args.join(' '));
    };

    originalExitCode =
      typeof process.exitCode === 'number' ? process.exitCode : undefined;
    process.exitCode = 0;
  });

  afterEach(() => {
    // eslint-disable-next-line no-console
    console.log = originalLog;
    process.exitCode = originalExitCode;
    fs.rmSync(tempDir, {recursive: true, force: true});
  });

  it('should detect identical directories with same filenames', () => {
    fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
    fs.writeFileSync(path.join(dir2, 'app.abc123.js'), 'var a=1;');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('All files have identical names'));
  });

  it('should detect file count mismatch', () => {
    fs.writeFileSync(path.join(dir1, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(dir2, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(dir2, 'file2.js'), 'content2');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    assert.equal(process.exitCode, 1);
    const output = consoleOutput.join('\n');
    assert(output.includes('File count mismatch'));
    assert(output.includes('Directory 1: 1 files'));
    assert(output.includes('Directory 2: 2 files'));
  });

  it('should show files only in directory 1', () => {
    fs.writeFileSync(path.join(dir1, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(dir1, 'file2.js'), 'content2');
    fs.writeFileSync(path.join(dir2, 'file1.js'), 'content1');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('Files only in directory 1'));
    assert(output.includes('file2.js'));
  });

  it('should show files only in directory 2', () => {
    fs.writeFileSync(path.join(dir1, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(dir2, 'file1.js'), 'content1');
    fs.writeFileSync(path.join(dir2, 'file2.js'), 'content2');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('Files only in directory 2'));
    assert(output.includes('file2.js'));
  });

  it('should match files by prefix when names differ', () => {
    fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
    fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var a=1;');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('Summary'));
    assert(output.includes('Identical files: 1'));
  });

  it('should detect differences in matched files', () => {
    fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
    fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var b=2;');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('Different files: 1'));
  });

  it('should work in summary mode', () => {
    fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
    fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var b=2;');

    const context = createContext(undefined, undefined, dir1, dir2, {
      summaryMode: true,
    });
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('Summary'));
    assert(output.includes('Different files: 1'));
    // Should not print full diff in summary mode
    assert(!output.includes('Comparing files'));
  });

  it('should work in verbose mode', () => {
    fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
    fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var a=1;');

    const context = createContext(undefined, undefined, dir1, dir2, {
      verbose: true,
    });
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    // Verbose mode should show all matches
    assert(output.includes('Comparing'));
  });

  it('should handle nested directories', () => {
    const subDir1 = path.join(dir1, 'subdir');
    const subDir2 = path.join(dir2, 'subdir');
    fs.mkdirSync(subDir1);
    fs.mkdirSync(subDir2);
    fs.writeFileSync(path.join(subDir1, 'file.js'), 'content');
    fs.writeFileSync(path.join(subDir2, 'file.js'), 'content');

    const context = createContext(undefined, undefined, dir1, dir2);
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    // Should complete without error - check for either summary or identical message
    assert(
      output.includes('Summary') ||
        output.includes('identical') ||
        output.includes('Comparing directories'),
    );
  });

  it('should use size threshold for matching', () => {
    fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
    fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var a=1;');

    const context = createContext(undefined, undefined, dir1, dir2, {
      sizeThreshold: 0.01,
    });
    compareDirectories(dir1, dir2, context);

    const output = consoleOutput.join('\n');
    assert(output.includes('Summary'));
  });
});
