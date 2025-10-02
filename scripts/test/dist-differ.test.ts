import {describe, it, beforeEach, afterEach} from 'mocha';
import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {
  stripContentHash,
  isWithinSizeThreshold,
  findMatchingFile,
  getAllFiles,
  compareFileContents,
  formatMinifiedJs,
  isJavaScriptFile,
  FileInfo,
  MatchingOptions
} from '../dist-differ-lib';

describe('dist-differ', () => {
  let tempDir: string;
  let testDir1: string;
  let testDir2: string;

  beforeEach(() => {
    // Create temporary directory for tests
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dist-differ-test-'));
    testDir1 = path.join(tempDir, 'dir1');
    testDir2 = path.join(tempDir, 'dir2');
    fs.mkdirSync(testDir1, {recursive: true});
    fs.mkdirSync(testDir2, {recursive: true});
  });

  afterEach(() => {
    // Clean up temporary directory
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, {recursive: true, force: true});
    }
  });

  describe('stripContentHash', () => {
    it('should strip simple hash from filename', () => {
      assert.strictEqual(stripContentHash('MyBundle.123456.js'), 'MyBundle.js');
      assert.strictEqual(stripContentHash('styles.abcdef.css'), 'styles.css');
      assert.strictEqual(stripContentHash('script.a1b2c3d4.min.js'), 'script.min.js');
    });

    it('should handle file paths with directories', () => {
      assert.strictEqual(stripContentHash('assets/bundle.123456.js'), 'assets/bundle.js');
      assert.strictEqual(stripContentHash('deep/nested/file.abcdef.css'), 'deep/nested/file.css');
    });

    it('should preserve files without hashes', () => {
      assert.strictEqual(stripContentHash('script.js'), 'script.js');
      assert.strictEqual(stripContentHash('styles.css'), 'styles.css');
      assert.strictEqual(stripContentHash('index.html'), 'index.html');
    });

    it('should not strip short strings (less than 4 chars)', () => {
      assert.strictEqual(stripContentHash('file.123.js'), 'file.123.js');
      assert.strictEqual(stripContentHash('test.ab.css'), 'test.ab.css');
    });

    it('should handle multiple dots in filename', () => {
      assert.strictEqual(stripContentHash('file.name.more.abcdef12.js'), 'file.name.more.js');
      assert.strictEqual(stripContentHash('jquery.min.1234567890.js'), 'jquery.min.js');
    });

    it('should handle mixed case hashes', () => {
      assert.strictEqual(stripContentHash('bundle.A1B2C3D4.js'), 'bundle.js');
      assert.strictEqual(stripContentHash('styles.AbCdEf12.css'), 'styles.css');
    });

    it('should preserve legitimate version numbers', () => {
      // These should NOT be stripped as they're likely version numbers
      assert.strictEqual(stripContentHash('jquery.1.2.3.js'), 'jquery.1.2.3.js');
      assert.strictEqual(stripContentHash('react.16.8.6.min.js'), 'react.16.8.6.min.js');
    });
  });

  describe('isWithinSizeThreshold', () => {
    it('should return true for identical sizes', () => {
      assert.strictEqual(isWithinSizeThreshold(100, 100, 5), true);
      assert.strictEqual(isWithinSizeThreshold(0, 0, 5), true);
    });

    it('should return true for sizes within threshold', () => {
      assert.strictEqual(isWithinSizeThreshold(100, 105, 10), true); // 5% difference
      assert.strictEqual(isWithinSizeThreshold(100, 95, 10), true);  // 5% difference
      assert.strictEqual(isWithinSizeThreshold(1000, 950, 10), true); // 5% difference
    });

    it('should return false for sizes outside threshold', () => {
      assert.strictEqual(isWithinSizeThreshold(100, 120, 10), false); // 20% difference
      assert.strictEqual(isWithinSizeThreshold(100, 80, 10), false);  // 20% difference
    });

    it('should handle zero sizes correctly', () => {
      assert.strictEqual(isWithinSizeThreshold(0, 100, 5), false);
      assert.strictEqual(isWithinSizeThreshold(100, 0, 5), false);
    });

    it('should calculate percentage based on larger file', () => {
      // 10 bytes difference from 100 bytes = 10%
      assert.strictEqual(isWithinSizeThreshold(100, 90, 15), true);
      assert.strictEqual(isWithinSizeThreshold(100, 90, 5), false);
      
      // Same absolute difference, but from smaller base
      assert.strictEqual(isWithinSizeThreshold(90, 100, 15), true);
      assert.strictEqual(isWithinSizeThreshold(90, 100, 5), false);
    });
  });

  describe('getAllFiles', () => {
    beforeEach(() => {
      // Create test files
      fs.writeFileSync(path.join(testDir1, 'file1.js'), 'content1');
      fs.writeFileSync(path.join(testDir1, 'file2.css'), 'content2');
      
      // Create subdirectory with files
      const subDir = path.join(testDir1, 'subdir');
      fs.mkdirSync(subDir);
      fs.writeFileSync(path.join(subDir, 'nested.js'), 'nested content');
      
      // Create a .js.map file that should be ignored
      fs.writeFileSync(path.join(testDir1, 'file1.js.map'), 'sourcemap content');
    });

    it('should find all files recursively', () => {
      const files = getAllFiles(testDir1);
      
      assert.strictEqual(files.length, 3); // Should not include .js.map file
      
      const relativePaths = files.map(f => f.relativePath).sort();
      assert.deepStrictEqual(relativePaths, [
        'file1.js',
        'file2.css',
        'subdir/nested.js'
      ]);
    });

    it('should include file sizes', () => {
      const files = getAllFiles(testDir1);
      
      files.forEach(file => {
        assert.strictEqual(typeof file.size, 'number');
        assert.ok(file.size > 0);
      });
    });

    it('should include normalized paths', () => {
      // Create file with hash
      fs.writeFileSync(path.join(testDir1, 'bundle.abc123.js'), 'bundled content');
      
      const files = getAllFiles(testDir1);
      const bundleFile = files.find(f => f.relativePath === 'bundle.abc123.js');
      
      assert.ok(bundleFile);
      assert.strictEqual(bundleFile!.normalizedPath, 'bundle.js');
    });

    it('should skip .js.map files', () => {
      const files = getAllFiles(testDir1);
      const mapFiles = files.filter(f => f.relativePath.endsWith('.js.map'));
      
      assert.strictEqual(mapFiles.length, 0);
    });
  });

  describe('findMatchingFile', () => {
    const options: MatchingOptions = { sizeThresholdPercent: 10 };
    
    it('should find exact path matches first', () => {
      const targetFile: FileInfo = {
        relativePath: 'bundle.js',
        fullPath: '/path/bundle.js',
        size: 100,
        normalizedPath: 'bundle.js'
      };
      
      const candidates: FileInfo[] = [
        {
          relativePath: 'bundle.abc123.js',
          fullPath: '/path2/bundle.abc123.js',
          size: 100,
          normalizedPath: 'bundle.js'
        },
        {
          relativePath: 'bundle.js',
          fullPath: '/path2/bundle.js',
          size: 200,
          normalizedPath: 'bundle.js'
        }
      ];
      
      const match = findMatchingFile(targetFile, candidates, options);
      assert.strictEqual(match?.relativePath, 'bundle.js');
    });

    it('should find normalized path matches when no exact match', () => {
      const targetFile: FileInfo = {
        relativePath: 'bundle.abc123.js',
        fullPath: '/path/bundle.abc123.js',
        size: 100,
        normalizedPath: 'bundle.js'
      };
      
      const candidates: FileInfo[] = [
        {
          relativePath: 'bundle.def456.js',
          fullPath: '/path2/bundle.def456.js',
          size: 100,
          normalizedPath: 'bundle.js'
        },
        {
          relativePath: 'other.js',
          fullPath: '/path2/other.js',
          size: 100,
          normalizedPath: 'other.js'
        }
      ];
      
      const match = findMatchingFile(targetFile, candidates, options);
      assert.strictEqual(match?.relativePath, 'bundle.def456.js');
    });

    it('should prefer closest size when multiple normalized matches', () => {
      const targetFile: FileInfo = {
        relativePath: 'bundle.abc123.js',
        fullPath: '/path/bundle.abc123.js',
        size: 100,
        normalizedPath: 'bundle.js'
      };
      
      const candidates: FileInfo[] = [
        {
          relativePath: 'bundle.def456.js',
          fullPath: '/path2/bundle.def456.js',
          size: 200, // 100 bytes difference
          normalizedPath: 'bundle.js'
        },
        {
          relativePath: 'bundle.xyz789.js',
          fullPath: '/path2/bundle.xyz789.js',
          size: 105, // 5 bytes difference (closer)
          normalizedPath: 'bundle.js'
        }
      ];
      
      const match = findMatchingFile(targetFile, candidates, options);
      assert.strictEqual(match?.relativePath, 'bundle.xyz789.js');
    });

    it('should return null when no matches found', () => {
      const targetFile: FileInfo = {
        relativePath: 'bundle.js',
        fullPath: '/path/bundle.js',
        size: 100,
        normalizedPath: 'bundle.js'
      };
      
      const candidates: FileInfo[] = [
        {
          relativePath: 'other.js',
          fullPath: '/path2/other.js',
          size: 100,
          normalizedPath: 'other.js'
        }
      ];
      
      const match = findMatchingFile(targetFile, candidates, options);
      assert.strictEqual(match, null);
    });
  });

  describe('formatMinifiedJs', () => {
    it('should format minified JavaScript with newlines', () => {
      // Create a longer minified string that will trigger formatting
      const minified = 'function test(){var a=1;var b=2;return a+b;}function test2(){var c=3;var d=4;return c+d;}console.log("hello");'.repeat(3);
      const formatted = formatMinifiedJs(minified);
      
      // Should add newlines after braces and semicolons
      assert.ok(formatted.includes('{\n'));
      assert.ok(formatted.includes(';\n'));
      assert.ok(formatted.includes('\n}'));
      
      // Should have more lines than the original
      const originalLines = minified.split('\n').length;
      const formattedLines = formatted.split('\n').length;
      assert.ok(formattedLines > originalLines);
    });

    it('should handle already formatted code', () => {
      const wellFormatted = `function test() {
  var a = 1;
  var b = 2;
  return a + b;
}`;
      
      const result = formatMinifiedJs(wellFormatted);
      // Should not significantly change already formatted code
      assert.ok(result.length < wellFormatted.length * 1.5);
    });

    it('should handle strings and avoid breaking them', () => {
      const withStrings = 'var msg="Hello;world";console.log(msg);';
      const formatted = formatMinifiedJs(withStrings);
      
      // Should still contain the string intact
      assert.ok(formatted.includes('"Hello;world"'));
    });

    it('should handle empty or short content', () => {
      assert.strictEqual(formatMinifiedJs(''), '');
      assert.strictEqual(formatMinifiedJs('var x = 1;'), 'var x = 1;');
    });

    it('should detect and format truly minified content', () => {
      // Long single line that should be formatted
      const longMinified = 'function a(){return 1;}function b(){return 2;}function c(){return 3;}'.repeat(10);
      const formatted = formatMinifiedJs(longMinified);
      
      // Should add many newlines to break up the long line
      const originalLines = longMinified.split('\n').length;
      const formattedLines = formatted.split('\n').length;
      assert.ok(formattedLines > originalLines * 3);
    });
  });

  describe('isJavaScriptFile', () => {
    it('should detect JavaScript files by extension', () => {
      assert.strictEqual(isJavaScriptFile('bundle.js'), true);
      assert.strictEqual(isJavaScriptFile('module.mjs'), true);
      assert.strictEqual(isJavaScriptFile('component.jsx'), true);
      assert.strictEqual(isJavaScriptFile('script.ts'), true);
      assert.strictEqual(isJavaScriptFile('component.tsx'), true);
    });

    it('should reject non-JavaScript files by extension', () => {
      assert.strictEqual(isJavaScriptFile('styles.css'), false);
      assert.strictEqual(isJavaScriptFile('data.json'), false);
      assert.strictEqual(isJavaScriptFile('page.html'), false);
      assert.strictEqual(isJavaScriptFile('image.png'), false);
    });

    it('should detect JavaScript by content patterns', () => {
      const jsContent = 'function test() { var x = 1; return x; }';
      assert.strictEqual(isJavaScriptFile('unknown.txt', jsContent), true);
      
      const moduleContent = 'module.exports = { test: function() {} };';
      assert.strictEqual(isJavaScriptFile('unknown', moduleContent), true);
      
      const es6Content = 'const test = () => { return "hello"; };';
      assert.strictEqual(isJavaScriptFile('unknown', es6Content), true);
    });

    it('should reject non-JavaScript content', () => {
      const cssContent = 'body { color: red; background: blue; }';
      assert.strictEqual(isJavaScriptFile('unknown.txt', cssContent), false);
      
      const htmlContent = '<html><body><h1>Hello</h1></body></html>';
      assert.strictEqual(isJavaScriptFile('unknown', htmlContent), false);
    });

    it('should prioritize extension over content', () => {
      // Even if content doesn't look like JS, trust the extension
      const cssContent = 'body { color: red; }';
      assert.strictEqual(isJavaScriptFile('bundle.js', cssContent), true);
    });
  });

  describe('compareFileContents', () => {
    let file1Path: string;
    let file2Path: string;

    beforeEach(() => {
      file1Path = path.join(testDir1, 'test1.txt');
      file2Path = path.join(testDir2, 'test2.txt');
    });

    it('should return null for identical files', () => {
      const content = 'Hello, world!';
      fs.writeFileSync(file1Path, content);
      fs.writeFileSync(file2Path, content);
      
      const result = compareFileContents(file1Path, file2Path);
      assert.strictEqual(result, null);
    });

    it('should return diff output for different files', () => {
      fs.writeFileSync(file1Path, 'Hello, world!');
      fs.writeFileSync(file2Path, 'Hello, universe!');
      
      const result = compareFileContents(file1Path, file2Path);
      assert.strictEqual(typeof result, 'string');
      assert.ok(result!.includes('world'));
      assert.ok(result!.includes('universe'));
    });

    it('should handle binary files gracefully', () => {
      // Create binary content
      const buffer1 = Buffer.from([0x00, 0x01, 0x02, 0x03]);
      const buffer2 = Buffer.from([0x00, 0x01, 0x02, 0x04]);
      
      fs.writeFileSync(file1Path, buffer1);
      fs.writeFileSync(file2Path, buffer2);
      
      const result = compareFileContents(file1Path, file2Path);
      assert.strictEqual(typeof result, 'string');
      assert.ok(result!.includes('differ'));
    });

    it('should handle missing files', () => {
      fs.writeFileSync(file1Path, 'content');
      // file2Path doesn't exist
      
      const result = compareFileContents(file1Path, '/nonexistent/file');
      assert.strictEqual(typeof result, 'string');
      assert.ok(result!.includes('Error'));
    });

    it('should format minified JavaScript for better diffs', () => {
      // Create minified JavaScript files
      const minifiedJs1 = 'function test(){var a=1;var b=2;return a+b;}console.log("hello");';
      const minifiedJs2 = 'function test(){var a=1;var c=3;return a+c;}console.log("hello");';
      
      const jsFile1 = path.join(testDir1, 'bundle.min.js');
      const jsFile2 = path.join(testDir2, 'bundle.min.js');
      
      fs.writeFileSync(jsFile1, minifiedJs1);
      fs.writeFileSync(jsFile2, minifiedJs2);
      
      const result = compareFileContents(jsFile1, jsFile2);
      assert.strictEqual(typeof result, 'string');
      // Should contain formatted diff with newlines, making differences more readable
      assert.ok(result!.includes('\n'));
      // Should show the actual variable name differences
      assert.ok(result!.includes('b') || result!.includes('c'));
    });

    it('should skip formatting for already-formatted JavaScript', () => {
      // Create well-formatted JavaScript files
      const formattedJs1 = `function test() {
  var a = 1;
  var b = 2;
  return a + b;
}
console.log("hello");`;
      
      const formattedJs2 = `function test() {
  var a = 1;
  var c = 3;
  return a + c;
}
console.log("hello");`;
      
      const jsFile1 = path.join(testDir1, 'bundle.js');
      const jsFile2 = path.join(testDir2, 'bundle.js');
      
      fs.writeFileSync(jsFile1, formattedJs1);
      fs.writeFileSync(jsFile2, formattedJs2);
      
      const result = compareFileContents(jsFile1, jsFile2);
      assert.strictEqual(typeof result, 'string');
      // Should still produce a readable diff
      assert.ok(result!.includes('b') || result!.includes('c'));
    });

    it('should handle non-JavaScript files normally', () => {
      // Create CSS files (non-JS)
      const css1 = 'body{color:red;background:blue;}';
      const css2 = 'body{color:green;background:blue;}';
      
      const cssFile1 = path.join(testDir1, 'styles.css');
      const cssFile2 = path.join(testDir2, 'styles.css');
      
      fs.writeFileSync(cssFile1, css1);
      fs.writeFileSync(cssFile2, css2);
      
      const result = compareFileContents(cssFile1, cssFile2);
      assert.strictEqual(typeof result, 'string');
      // Should show differences without JS formatting
      assert.ok(result!.includes('red') || result!.includes('green'));
    });

    it('should handle very large minified files', () => {
      // Create a large minified file
      const largeMinified = 'var x=1;'.repeat(10000) + 'function test(){return x;}';
      const largeMinified2 = 'var x=2;'.repeat(10000) + 'function test(){return x;}';
      
      const largeFile1 = path.join(testDir1, 'large.min.js');
      const largeFile2 = path.join(testDir2, 'large.min.js');
      
      fs.writeFileSync(largeFile1, largeMinified);
      fs.writeFileSync(largeFile2, largeMinified2);
      
      const result = compareFileContents(largeFile1, largeFile2);
      assert.strictEqual(typeof result, 'string');
      // Should handle large files without crashing
      assert.ok(result!.length > 0);
    });
  });

  describe('integration tests', () => {
    beforeEach(() => {
      // Create test scenario with hashed files
      fs.writeFileSync(path.join(testDir1, 'bundle.abc123.js'), 'console.log("hello");');
      fs.writeFileSync(path.join(testDir1, 'styles.def456.css'), 'body { color: red; }');
      fs.writeFileSync(path.join(testDir1, 'vendor.xyz789.js'), 'var lib = {};');
      
      fs.writeFileSync(path.join(testDir2, 'bundle.uvw012.js'), 'console.log("hello");');
      fs.writeFileSync(path.join(testDir2, 'styles.rst345.css'), 'body { color: blue; }'); // Different content
      // vendor.js missing in dir2
      fs.writeFileSync(path.join(testDir2, 'newfile.js'), 'var x = 1;'); // Only in dir2
    });

    it('should correctly match files by normalized paths', () => {
      const files1 = getAllFiles(testDir1);
      const files2 = getAllFiles(testDir2);
      
      assert.strictEqual(files1.length, 3);
      assert.strictEqual(files2.length, 3);
      
      // Check that normalized paths are correct
      const bundle1 = files1.find(f => f.relativePath.includes('bundle'));
      const bundle2 = files2.find(f => f.relativePath.includes('bundle'));
      
      assert.strictEqual(bundle1?.normalizedPath, 'bundle.js');
      assert.strictEqual(bundle2?.normalizedPath, 'bundle.js');
    });

    it('should find matching pairs correctly', () => {
      const files1 = getAllFiles(testDir1);
      const files2 = getAllFiles(testDir2);
      const options: MatchingOptions = { sizeThresholdPercent: 5 };
      
      // Test bundle matching (should match - same content)
      const bundle1 = files1.find(f => f.relativePath.includes('bundle'))!;
      const bundleMatch = findMatchingFile(bundle1, files2, options);
      assert.notStrictEqual(bundleMatch, null);
      assert.strictEqual(bundleMatch?.normalizedPath, 'bundle.js');
      
      // Test styles matching (should match - different content but same normalized path)
      const styles1 = files1.find(f => f.relativePath.includes('styles'))!;
      const stylesMatch = findMatchingFile(styles1, files2, options);
      assert.notStrictEqual(stylesMatch, null);
      assert.strictEqual(stylesMatch?.normalizedPath, 'styles.css');
      
      // Test vendor (should not match - missing in dir2)
      const vendor1 = files1.find(f => f.relativePath.includes('vendor'))!;
      const vendorMatch = findMatchingFile(vendor1, files2, options);
      assert.strictEqual(vendorMatch, null);
    });
  });
});