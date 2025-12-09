import assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {compareFiles, compareFilesByPrefix} from '../src/comparison';
import {compareDirectories} from '../src/directory';
import {createContext} from '../src/context';
import type {JsonReport} from '../src/json';

describe('JSON output', () => {
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

  function parseJsonOutput(): JsonReport {
    const output = consoleOutput.join('\n');
    // Find JSON in output (might have other text before/after)
    const jsonMatch = output.match(/\{[\s\S]*\}/);
    if (!jsonMatch) {
      throw new Error('No JSON found in output');
    }
    return JSON.parse(jsonMatch[0]) as JsonReport;
  }

  describe('file comparison', () => {
    it('should output JSON for identical files', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'var a=1;');
      fs.writeFileSync(file2, 'var a=1;');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      assert.equal(report.metadata.file1, path.resolve(file1));
      assert.equal(report.metadata.file2, path.resolve(file2));
      assert.equal(report.summary.identical, true);
      assert.equal(report.summary.totalHunks, 0);
      assert.equal(report.files?.length, 1);
      assert.equal(report.files![0].status, 'identical');
    });

    it('should output JSON for different files', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'var a=1;');
      fs.writeFileSync(file2, 'var b=2;');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      assert.equal(report.summary.identical, false);
      assert(report.summary.totalHunks > 0);
      assert.equal(report.files?.length, 1);
      assert.equal(report.files![0].status, 'different');
      assert(report.files![0].hunks.length > 0);
    });

    it('should categorize hunks as meaningful or harmless', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'var a=1;');
      fs.writeFileSync(file2, 'var b=2;');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      const hunks = report.files![0].hunks;
      assert(hunks.length > 0);
      for (const hunk of hunks) {
        assert(['meaningful', 'harmless'].includes(hunk.category));
        if (hunk.category === 'harmless') {
          assert(hunk.harmlessType !== undefined);
        }
        assert(hunk.confidence >= 0 && hunk.confidence <= 1);
      }
    });

    it('should include context lines in hunks', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'var x=1;var a=2;var y=3;');
      fs.writeFileSync(file2, 'var x=1;var b=2;var y=3;');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      const hunks = report.files![0].hunks;
      if (hunks.length > 0) {
        const hunk = hunks[0];
        assert(Array.isArray(hunk.context.before));
        assert(Array.isArray(hunk.context.after));
      }
    });

    it('should include normalized representations for harmless changes', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'var a="abc12";');
      fs.writeFileSync(file2, 'var a="def34";');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
        ignoreAssetIds: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      const hunks = report.files![0].hunks;
      for (const hunk of hunks) {
        if (hunk.category === 'harmless' && hunk.harmlessType === 'asset_ids') {
          assert(hunk.normalized !== undefined);
          assert.equal(hunk.normalized.before, hunk.normalized.after);
        }
      }
    });

    it('should include analysis for meaningful changes', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'function old() { return null; }');
      fs.writeFileSync(file2, 'function new() { return { data: "value" }; }');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      const meaningfulHunks = report.files![0].hunks.filter(
        (h) => h.category === 'meaningful',
      );
      if (meaningfulHunks.length > 0) {
        const hunk = meaningfulHunks[0];
        assert(hunk.analysis !== undefined);
        assert.equal(hunk.analysis.semanticChange, true);
        assert(hunk.analysis.changeType !== undefined);
        assert(['low', 'medium', 'high'].includes(hunk.analysis.impact!));
      }
    });
  });

  describe('directory comparison', () => {
    it('should output JSON for directory comparison', () => {
      fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
      fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var a=1;');

      const context = createContext(undefined, undefined, dir1, dir2, {
        jsonMode: true,
      });
      compareDirectories(dir1, dir2, context);

      const report = parseJsonOutput();
      assert.equal(report.metadata.dir1, path.resolve(dir1));
      assert.equal(report.metadata.dir2, path.resolve(dir2));
      assert(report.summary.totalFiles !== undefined);
      assert(report.summary.identicalFiles !== undefined);
      assert(report.summary.differentFiles !== undefined);
      assert(Array.isArray(report.files));
    });

    it('should include file results in directory JSON', () => {
      fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
      fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var b=2;');

      const context = createContext(undefined, undefined, dir1, dir2, {
        jsonMode: true,
      });
      compareDirectories(dir1, dir2, context);

      const report = parseJsonOutput();
      assert(report.files!.length > 0);
      for (const file of report.files!) {
        assert(file.path !== undefined);
        assert(['identical', 'different'].includes(file.status));
        assert(typeof file.hunkCount === 'number');
        assert(typeof file.meaningfulHunkCount === 'number');
        assert(typeof file.harmlessHunkCount === 'number');
      }
    });

    it('should handle file count mismatch in JSON', () => {
      fs.writeFileSync(path.join(dir1, 'file1.js'), 'content1');
      fs.writeFileSync(path.join(dir2, 'file1.js'), 'content1');
      fs.writeFileSync(path.join(dir2, 'file2.js'), 'content2');

      const context = createContext(undefined, undefined, dir1, dir2, {
        jsonMode: true,
      });
      compareDirectories(dir1, dir2, context);

      const report = parseJsonOutput();
      assert.equal(report.summary.error, 'file_count_mismatch');
      assert.equal(report.summary.files1Count, 1);
      assert.equal(report.summary.files2Count, 2);
      assert.equal(process.exitCode, 1);
    });

    it('should include ambiguous matches in JSON', () => {
      // Create multiple files with same prefix but different sizes
      fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
      fs.writeFileSync(path.join(dir1, 'app.def456.js'), 'var a=1;var b=2;');
      fs.writeFileSync(path.join(dir2, 'app.ghi789.js'), 'var a=1;');
      fs.writeFileSync(path.join(dir2, 'app.jkl012.js'), 'var a=1;var b=2;');

      const context = createContext(undefined, undefined, dir1, dir2, {
        jsonMode: true,
        sizeThreshold: 0.01,
      });
      compareDirectories(dir1, dir2, context);

      const report = parseJsonOutput();
      // Should have ambiguous matches or successfully matched files
      assert(
        (report.ambiguousMatches && report.ambiguousMatches.length > 0) ||
          report.files!.length > 0,
      );
    });

    it('should respect ignore options in JSON metadata', () => {
      fs.writeFileSync(path.join(dir1, 'app.abc123.js'), 'var a=1;');
      fs.writeFileSync(path.join(dir2, 'app.def456.js'), 'var b=2;');

      const context = createContext(undefined, undefined, dir1, dir2, {
        jsonMode: true,
        ignoreAssetIds: true,
        ignoreUnminifiedRefs: true,
        ignoreSourceMapUrl: true,
        ignoreSwappedVariables: true,
      });
      compareDirectories(dir1, dir2, context);

      const report = parseJsonOutput();
      assert.equal(report.metadata.options.ignoreAssetIds, true);
      assert.equal(report.metadata.options.ignoreUnminifiedRefs, true);
      assert.equal(report.metadata.options.ignoreSourceMapUrl, true);
      assert.equal(report.metadata.options.ignoreSwappedVariables, true);
    });

    it('should filter harmless hunks when ignore options are enabled', () => {
      const file1 = path.join(dir1, 'file1.js');
      const file2 = path.join(dir2, 'file2.js');
      fs.writeFileSync(file1, 'var a="abc12";');
      fs.writeFileSync(file2, 'var a="def34";');

      const context = createContext(file1, file2, undefined, undefined, {
        jsonMode: true,
        ignoreAssetIds: true,
      });
      compareFiles(file1, file2, context);

      const report = parseJsonOutput();
      // With ignoreAssetIds enabled, asset ID differences should be categorized as harmless
      const harmlessHunks = report.files![0].hunks.filter(
        (h) => h.category === 'harmless',
      );
      assert(harmlessHunks.length > 0);
      assert(harmlessHunks.some((h) => h.harmlessType === 'asset_ids'));
    });
  });
});
