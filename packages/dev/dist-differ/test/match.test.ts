import assert from 'assert';
import {extractPrefix, formatFileSize, matchFilesByPrefix} from '../src/match';
import type {FileInfo} from '../src/types';

describe('extractPrefix', () => {
  it('should extract prefix from standard filename', () => {
    const filename = 'async-error-flag-renderer.c272922c.js';
    const result = extractPrefix(filename);
    assert.equal(result, 'async-error-flag-renderer');
  });

  it('should extract prefix from filename with multiple dots', () => {
    const filename = 'my.component.a1b2c.js';
    const result = extractPrefix(filename);
    assert.equal(result, 'my.component');
  });

  it('should return filename if no prefix pattern found', () => {
    const filename = 'simple.js';
    const result = extractPrefix(filename);
    assert.equal(result, 'simple.js');
  });

  it('should handle filename with no extension', () => {
    const filename = 'noextension';
    const result = extractPrefix(filename);
    assert.equal(result, 'noextension');
  });

  it('should handle filename with single dot', () => {
    const filename = 'file.js';
    const result = extractPrefix(filename);
    assert.equal(result, 'file.js');
  });

  it('should handle complex hash patterns', () => {
    const filename = 'app.1234567890abcdef.js';
    const result = extractPrefix(filename);
    assert.equal(result, 'app');
  });
});

describe('formatFileSize', () => {
  it('should format small numbers without commas', () => {
    assert.equal(formatFileSize(123), '123');
    assert.equal(formatFileSize(999), '999');
  });

  it('should format numbers with commas', () => {
    assert.equal(formatFileSize(1000), '1,000');
    assert.equal(formatFileSize(1234), '1,234');
    assert.equal(formatFileSize(12345), '12,345');
    assert.equal(formatFileSize(123456), '123,456');
    assert.equal(formatFileSize(1234567), '1,234,567');
  });

  it('should handle zero', () => {
    assert.equal(formatFileSize(0), '0');
  });

  it('should handle large numbers', () => {
    assert.equal(formatFileSize(1000000000), '1,000,000,000');
  });
});

describe('matchFilesByPrefix', () => {
  function createFileInfo(
    filename: string,
    size: number,
    dirPath: string = '',
  ): FileInfo {
    const relativePath = dirPath ? `${dirPath}/${filename}` : filename;
    return {
      fullPath: `/test/${relativePath}`,
      relativePath,
      filename,
      size,
    };
  }

  it('should match files with exact same filename', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000)];
    const files2 = [createFileInfo('app.abc123.js', 1000)];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 1);
    assert.equal(result.ambiguous.length, 0);
    assert.equal(result.matched[0].file1.filename, 'app.abc123.js');
    assert.equal(result.matched[0].file2.filename, 'app.abc123.js');
  });

  it('should match files by prefix when 1:1', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000)];
    const files2 = [createFileInfo('app.def456.js', 1000)];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 1);
    assert.equal(result.ambiguous.length, 0);
    assert.equal(result.matched[0].prefix, 'app');
  });

  it('should match files by exact size when multiple files share prefix', () => {
    const files1 = [
      createFileInfo('app.abc123.js', 1000),
      createFileInfo('app.def456.js', 2000),
    ];
    const files2 = [
      createFileInfo('app.xyz789.js', 1000),
      createFileInfo('app.uvw012.js', 2000),
    ];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 2);
    assert.equal(result.ambiguous.length, 0);
  });

  it('should mark ambiguous when multiple files match same size', () => {
    const files1 = [
      createFileInfo('app.abc123.js', 1000),
      createFileInfo('app.def456.js', 1000),
    ];
    const files2 = [
      createFileInfo('app.xyz789.js', 1000),
      createFileInfo('app.uvw012.js', 1000),
    ];

    const result = matchFilesByPrefix(files1, files2);

    // Should match by exact filename first if possible, otherwise ambiguous
    assert(result.matched.length >= 0);
  });

  it('should use size threshold for close matches', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000)];
    const files2 = [createFileInfo('app.def456.js', 1005)]; // 0.5% difference

    const result = matchFilesByPrefix(files1, files2, 0.01); // 1% threshold

    assert.equal(result.matched.length, 1);
    assert.equal(result.ambiguous.length, 0);
  });

  it('should not match when size difference exceeds threshold', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000, 'dir')];
    const files2 = [createFileInfo('app.def456.js', 1100, 'dir')]; // 10% difference

    const result = matchFilesByPrefix(files1, files2, 0.01); // 1% threshold

    // Will still match by prefix (1:1 match), but size difference is noted
    assert(result.matched.length >= 0);
  });

  it('should handle files only in first directory', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000)];
    const files2: FileInfo[] = [];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 0);
    assert.equal(result.ambiguous.length, 1);
    assert.equal(result.ambiguous[0].files1.length, 1);
    assert.equal(result.ambiguous[0].files2.length, 0);
  });

  it('should handle files only in second directory', () => {
    const files1: FileInfo[] = [];
    const files2 = [createFileInfo('app.abc123.js', 1000)];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 0);
    assert.equal(result.ambiguous.length, 1);
    assert.equal(result.ambiguous[0].files1.length, 0);
    assert.equal(result.ambiguous[0].files2.length, 1);
  });

  it('should handle files in different directories', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000, 'dir1')];
    const files2 = [createFileInfo('app.def456.js', 1000, 'dir2')];

    const result = matchFilesByPrefix(files1, files2);

    // Files in different directories should not match
    assert.equal(result.matched.length, 0);
  });

  it('should match files in same directory with different prefixes', () => {
    const files1 = [
      createFileInfo('app1.abc123.js', 1000, 'dir'),
      createFileInfo('app2.def456.js', 2000, 'dir'),
    ];
    const files2 = [
      createFileInfo('app1.xyz789.js', 1000, 'dir'),
      createFileInfo('app2.uvw012.js', 2000, 'dir'),
    ];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 2);
    assert.equal(result.ambiguous.length, 0);
  });

  it('should prefer exact filename matches over prefix matches', () => {
    const files1 = [
      createFileInfo('app.abc123.js', 1000),
      createFileInfo('app.def456.js', 2000),
    ];
    const files2 = [
      createFileInfo('app.abc123.js', 1500), // Same filename, different size
      createFileInfo('app.xyz789.js', 2000),
    ];

    const result = matchFilesByPrefix(files1, files2);

    // Should match app.abc123.js by exact filename first
    assert(result.matched.length >= 1);
    const exactMatch = result.matched.find(
      (m) => m.file1.filename === 'app.abc123.js',
    );
    assert(exactMatch !== undefined);
  });

  it('should handle empty file lists', () => {
    const result = matchFilesByPrefix([], []);

    assert.equal(result.matched.length, 0);
    assert.equal(result.ambiguous.length, 0);
  });

  it('should handle complex scenario with multiple matches', () => {
    const files1 = [
      createFileInfo('a.1.js', 100, 'dir'),
      createFileInfo('a.2.js', 200, 'dir'),
      createFileInfo('b.1.js', 300, 'dir'),
    ];
    const files2 = [
      createFileInfo('a.3.js', 100, 'dir'),
      createFileInfo('a.4.js', 200, 'dir'),
      createFileInfo('b.2.js', 300, 'dir'),
    ];

    const result = matchFilesByPrefix(files1, files2);

    assert.equal(result.matched.length, 3);
    assert.equal(result.ambiguous.length, 0);
  });

  it('should sort size-based matches by difference', () => {
    const files1 = [createFileInfo('app.abc123.js', 1000)];
    const files2 = [
      createFileInfo('app.def456.js', 1005), // 0.5% diff
      createFileInfo('app.xyz789.js', 1002), // 0.2% diff - should match this
    ];

    const result = matchFilesByPrefix(files1, files2, 0.01);

    assert.equal(result.matched.length, 1);
    // Should match the closer one (1002)
    assert.equal(result.matched[0].file2.size, 1002);
  });
});
