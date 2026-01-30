import assert from 'assert';
import {parseArgs, printUsage} from '../src/cli';

describe('parseArgs', () => {
  it('should parse no arguments', () => {
    const result = parseArgs([]);

    assert.equal(result.files.length, 0);
    assert.equal(result.options.ignoreAssetIds, false);
    assert.equal(result.options.ignoreUnminifiedRefs, false);
    assert.equal(result.options.ignoreSourceMapUrl, false);
    assert.equal(result.options.ignoreSwappedVariables, false);
    assert.equal(result.options.summaryMode, false);
    assert.equal(result.options.verbose, false);
    assert.equal(result.options.sizeThreshold, 0.01);
  });

  it('should parse file arguments', () => {
    const result = parseArgs(['file1.js', 'file2.js']);

    assert.equal(result.files.length, 2);
    assert.equal(result.files[0], 'file1.js');
    assert.equal(result.files[1], 'file2.js');
  });

  it('should parse --ignore-asset-ids flag', () => {
    const result = parseArgs(['--ignore-asset-ids', 'file1.js', 'file2.js']);

    assert.equal(result.options.ignoreAssetIds, true);
    assert.equal(result.files.length, 2);
  });

  it('should parse --ignore-unminified-refs flag', () => {
    const result = parseArgs([
      '--ignore-unminified-refs',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.ignoreUnminifiedRefs, true);
  });

  it('should parse --ignore-source-map-url flag', () => {
    const result = parseArgs([
      '--ignore-source-map-url',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.ignoreSourceMapUrl, true);
  });

  it('should parse --ignore-swapped-variables flag', () => {
    const result = parseArgs([
      '--ignore-swapped-variables',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.ignoreSwappedVariables, true);
  });

  it('should parse --ignore-all flag', () => {
    const result = parseArgs(['--ignore-all', 'file1.js', 'file2.js']);

    assert.equal(result.options.ignoreAssetIds, true);
    assert.equal(result.options.ignoreUnminifiedRefs, true);
    assert.equal(result.options.ignoreSourceMapUrl, true);
    assert.equal(result.options.ignoreSwappedVariables, true);
  });

  it('should parse --summary flag', () => {
    const result = parseArgs(['--summary', 'file1.js', 'file2.js']);

    assert.equal(result.options.summaryMode, true);
  });

  it('should parse --verbose flag', () => {
    const result = parseArgs(['--verbose', 'file1.js', 'file2.js']);

    assert.equal(result.options.verbose, true);
  });

  it('should parse --disambiguation-size-threshold with value', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      '0.05',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.sizeThreshold, 0.05);
  });

  it('should return error when --disambiguation-size-threshold missing value', () => {
    const result = parseArgs(['--disambiguation-size-threshold']);

    assert(result.error);
    assert(result.error.includes('requires a value'));
  });

  it('should return error when --disambiguation-size-threshold value is NaN', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      'invalid',
      'file1.js',
      'file2.js',
    ]);

    assert(result.error);
    assert(result.error.includes('must be a number'));
  });

  it('should return error when --disambiguation-size-threshold value is negative', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      '-0.1',
      'file1.js',
      'file2.js',
    ]);

    assert(result.error);
    assert(result.error.includes('must be a number between 0 and 1'));
  });

  it('should return error when --disambiguation-size-threshold value is greater than 1', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      '1.5',
      'file1.js',
      'file2.js',
    ]);

    assert(result.error);
    assert(result.error.includes('must be a number between 0 and 1'));
  });

  it('should accept threshold value of 0', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      '0',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.sizeThreshold, 0);
    assert(!result.error);
  });

  it('should accept threshold value of 1', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      '1',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.sizeThreshold, 1);
    assert(!result.error);
  });

  it('should parse multiple flags', () => {
    const result = parseArgs([
      '--ignore-asset-ids',
      '--ignore-unminified-refs',
      '--summary',
      '--verbose',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.options.ignoreAssetIds, true);
    assert.equal(result.options.ignoreUnminifiedRefs, true);
    assert.equal(result.options.summaryMode, true);
    assert.equal(result.options.verbose, true);
  });

  it('should return error for unknown flag', () => {
    const result = parseArgs(['--unknown-flag', 'file1.js', 'file2.js']);

    assert(result.error);
    assert(result.error.includes('Unknown flag'));
  });

  it('should handle flags in different orders', () => {
    const result = parseArgs(['file1.js', '--ignore-asset-ids', 'file2.js']);

    assert.equal(result.files.length, 2);
    assert.equal(result.files[0], 'file1.js');
    assert.equal(result.files[1], 'file2.js');
    assert.equal(result.options.ignoreAssetIds, true);
  });

  it('should skip threshold value when parsing files', () => {
    const result = parseArgs([
      '--disambiguation-size-threshold',
      '0.05',
      'file1.js',
      'file2.js',
    ]);

    assert.equal(result.files.length, 2);
    assert(!result.files.includes('0.05'));
  });
});

describe('printUsage', () => {
  let consoleOutput: string[];
  let originalError: typeof console.error;

  beforeEach(() => {
    consoleOutput = [];
    // eslint-disable-next-line no-console
    originalError = console.error;
    // eslint-disable-next-line no-console
    console.error = (...args: any[]) => {
      consoleOutput.push(args.join(' '));
    };
  });

  afterEach(() => {
    // eslint-disable-next-line no-console
    console.error = originalError;
  });

  it('should print usage information', () => {
    printUsage();

    const output = consoleOutput.join('\n');
    assert(output.includes('Usage:'));
    assert(output.includes('OPTIONS'));
    assert(output.includes('file1|dir1'));
    assert(output.includes('file2|dir2'));
  });

  it('should include option descriptions', () => {
    printUsage();

    const output = consoleOutput.join('\n');
    assert(output.includes('--ignore-all'));
    assert(output.includes('--ignore-asset-ids'));
    assert(output.includes('--ignore-unminified-refs'));
    assert(output.includes('--summary'));
    assert(output.includes('--verbose'));
    assert(output.includes('--disambiguation-size-threshold'));
  });

  it('should include examples', () => {
    printUsage();

    const output = consoleOutput.join('\n');
    assert(output.includes('Examples:'));
  });
});
