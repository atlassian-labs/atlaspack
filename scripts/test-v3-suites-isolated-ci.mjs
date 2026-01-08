#!/usr/bin/env node
/* eslint-disable no-console */

/**
 * CI version of test-v3-suites-isolated that runs each test suite in isolation
 * with full mocha output and clear delimiters to identify which suite is broken.
 *
 * This script:
 * 1. Finds all test files in packages/core/integration-tests/test/
 * 2. Splits tests into slices if SLICE_NUM and SLICE_TOTAL are provided
 * 3. Runs each test file separately with ATLASPACK_V3=true
 * 4. Shows full mocha output (stdio: inherit)
 * 5. Uses clear delimiters to mark the start/end of each suite
 * 6. Monitors for hangs and reports which suites hang or fail
 * 7. NO state management - runs all tests every time
 *
 * Usage:
 *   node scripts/test-v3-suites-isolated-ci.mjs
 *   SLICE_NUM=1 SLICE_TOTAL=2 node scripts/test-v3-suites-isolated-ci.mjs
 */

import {spawn} from 'child_process';
import {readdir, stat} from 'fs/promises';
import {join, relative} from 'path';
import {fileURLToPath} from 'url';
import {dirname} from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PROJECT_ROOT = join(__dirname, '..');
const TEST_DIR = join(PROJECT_ROOT, 'packages/core/integration-tests/test');
const HANG_TIMEOUT_MS = 30000; // 30 seconds
const MAX_TEST_TIMEOUT_MS = 300000; // 5 minutes max per test

// Directories to exclude from test discovery
const EXCLUDED_DIRS = new Set([
  'integration',
  'shutdown',
  'terminate-cleanly',
  'tmp',
  'project-specific-lockfiles',
]);

// Test file extensions
const TEST_EXTENSIONS = ['.ts', '.js', '.mjs', '.cjs', '.mts', '.cts'];

async function isDirectory(path) {
  try {
    const stats = await stat(path);
    return stats.isDirectory();
  } catch {
    return false;
  }
}

async function findTestFiles(dir, baseDir = dir) {
  const files = [];
  const entries = await readdir(dir);

  for (const entry of entries) {
    const fullPath = join(dir, entry);
    const relativePath = relative(baseDir, fullPath);
    const pathParts = relativePath.split('/');

    // Skip excluded directories
    if (pathParts.some((part) => EXCLUDED_DIRS.has(part))) {
      continue;
    }

    const isDir = await isDirectory(fullPath);
    if (isDir) {
      // Recursively search subdirectories
      const subFiles = await findTestFiles(fullPath, baseDir);
      files.push(...subFiles);
    } else {
      // Check if it's a test file
      const ext = entry.substring(entry.lastIndexOf('.'));
      if (TEST_EXTENSIONS.includes(ext)) {
        files.push(fullPath);
      }
    }
  }

  return files.sort();
}

function printDelimiter(text, char = '=') {
  const width = 80;
  const padding = Math.max(0, width - text.length - 4);
  const leftPad = Math.floor(padding / 2);
  const rightPad = padding - leftPad;
  console.log(char.repeat(width));
  console.log(
    `${char}${' '.repeat(leftPad)}${text}${' '.repeat(rightPad)}${char}`,
  );
  console.log(char.repeat(width));
}

// eslint-disable-next-line require-await
function runTestSuite(testFile) {
  return new Promise((resolve) => {
    const relativePath = relative(TEST_DIR, testFile);

    const testProcess = spawn(
      'yarn',
      ['workspace', '@atlaspack/integration-tests', 'test', testFile],
      {
        cwd: PROJECT_ROOT,
        env: {
          ...process.env,
          ATLASPACK_V3: 'true',
        },
        stdio: 'inherit', // Full mocha output
      },
    );

    let hangDetected = false;
    let startTime = Date.now();

    // Monitor for hangs by checking if process is still running
    const hangTimeout = setInterval(() => {
      if (testProcess.killed || testProcess.exitCode !== null) {
        clearInterval(hangTimeout);
        return;
      }

      const timeSinceStart = Date.now() - startTime;

      // If process has been running for more than MAX_TEST_TIMEOUT_MS, consider it hung
      if (timeSinceStart >= MAX_TEST_TIMEOUT_MS) {
        hangDetected = true;
        console.error(
          `\n⚠️  HANG DETECTED: Test has been running for ${MAX_TEST_TIMEOUT_MS / 1000}s`,
        );
        testProcess.kill('SIGTERM');
        clearInterval(hangTimeout);
        clearTimeout(maxTimeout);

        setTimeout(() => {
          if (!testProcess.killed) {
            testProcess.kill('SIGKILL');
          }
        }, 5000);
      }
    }, 1000);

    const maxTimeout = setTimeout(() => {
      if (!testProcess.killed) {
        hangDetected = true;
        console.error(`\n⚠️  HANG DETECTED: Test exceeded maximum timeout`);
        testProcess.kill('SIGTERM');
        clearInterval(hangTimeout);

        setTimeout(() => {
          if (!testProcess.killed) {
            testProcess.kill('SIGKILL');
          }
        }, 5000);
      }
    }, MAX_TEST_TIMEOUT_MS);

    testProcess.on('exit', (code, signal) => {
      clearInterval(hangTimeout);
      clearTimeout(maxTimeout);

      const result = {
        file: relativePath,
        success: code === 0 && !hangDetected,
        hangDetected,
        exitCode: code,
        signal,
        duration: Date.now() - startTime,
      };

      resolve(result);
    });

    testProcess.on('error', (error) => {
      clearInterval(hangTimeout);
      clearTimeout(maxTimeout);

      resolve({
        file: relativePath,
        success: false,
        hangDetected: false,
        exitCode: null,
        signal: null,
        duration: Date.now() - startTime,
        error: error.message,
      });
    });
  });
}

function getSlice(testFiles, sliceNum, sliceTotal) {
  if (!sliceNum || !sliceTotal) {
    return testFiles;
  }

  const sliceNumInt = parseInt(sliceNum, 10);
  const sliceTotalInt = parseInt(sliceTotal, 10);

  if (isNaN(sliceNumInt) || isNaN(sliceTotalInt) || sliceNumInt < 1 || sliceNumInt > sliceTotalInt) {
    throw new Error(
      `Invalid slice parameters: SLICE_NUM=${sliceNum}, SLICE_TOTAL=${sliceTotal}`,
    );
  }

  const totalFiles = testFiles.length;
  const filesPerSlice = Math.ceil(totalFiles / sliceTotalInt);
  const startIndex = (sliceNumInt - 1) * filesPerSlice;
  const endIndex = Math.min(startIndex + filesPerSlice, totalFiles);

  return testFiles.slice(startIndex, endIndex);
}

async function main() {
  const sliceNum = process.env.SLICE_NUM;
  const sliceTotal = process.env.SLICE_TOTAL;

  console.log('\n');
  if (sliceNum && sliceTotal) {
    printDelimiter(
      `V3 INTEGRATION TESTS - ISOLATED RUNNER (CI MODE) - SLICE ${sliceNum}/${sliceTotal}`,
      '=',
    );
  } else {
    printDelimiter('V3 INTEGRATION TESTS - ISOLATED RUNNER (CI MODE)', '=');
  }
  console.log('Running each test suite in isolation with full mocha output\n');

  console.log('Finding test files...');
  const allTestFiles = await findTestFiles(TEST_DIR);
  console.log(`Found ${allTestFiles.length} test files`);

  const testFiles = getSlice(allTestFiles, sliceNum, sliceTotal);

  if (sliceNum && sliceTotal) {
    console.log(
      `Running slice ${sliceNum}/${sliceTotal}: ${testFiles.length} test files\n`,
    );
  } else {
    console.log(`Running all ${testFiles.length} test files\n`);
  }

  const results = {
    passed: [],
    failed: [],
    hung: [],
    errors: [],
  };

  // Track which test number we're on within the slice
  for (let i = 0; i < testFiles.length; i++) {
    const testFile = testFiles[i];
    const relativePath = relative(TEST_DIR, testFile);

    console.log('\n');
    printDelimiter(`SUITE ${i + 1}/${testFiles.length}: ${relativePath}`, '=');
    console.log('');

    const result = await runTestSuite(testFile);

    console.log('');
    if (result.hangDetected) {
      results.hung.push(result);
      printDelimiter(`❌ HANG DETECTED: ${relativePath}`, '-');
    } else if (result.success) {
      results.passed.push(result);
      printDelimiter(
        `✅ PASSED: ${relativePath} (${(result.duration / 1000).toFixed(1)}s)`,
        '-',
      );
    } else if (result.error) {
      results.errors.push(result);
      printDelimiter(`❌ ERROR: ${relativePath} - ${result.error}`, '-');
    } else {
      results.failed.push(result);
      printDelimiter(
        `❌ FAILED: ${relativePath} (exit code: ${result.exitCode}, ${(result.duration / 1000).toFixed(1)}s)`,
        '-',
      );
    }
  }

  console.log('\n');
  if (sliceNum && sliceTotal) {
    printDelimiter(
      `FINAL SUMMARY - SLICE ${sliceNum}/${sliceTotal}`,
      '=',
    );
  } else {
    printDelimiter('FINAL SUMMARY', '=');
  }
  console.log(`Total suites in slice: ${testFiles.length}`);
  if (sliceNum && sliceTotal) {
    console.log(`Total suites overall: ${allTestFiles.length}`);
  }
  console.log(`✅ Passed: ${results.passed.length}`);
  console.log(`❌ Failed: ${results.failed.length}`);
  console.log(`⚠️  Hung: ${results.hung.length}`);
  console.log(`💥 Errors: ${results.errors.length}`);

  if (results.hung.length > 0) {
    console.log('\n');
    printDelimiter('HANGING TEST SUITES', '=');
    for (const result of results.hung) {
      console.log(`  ⚠️  ${result.file}`);
    }
  }

  if (results.failed.length > 0) {
    console.log('\n');
    printDelimiter('FAILED TEST SUITES', '=');
    for (const result of results.failed) {
      console.log(`  ❌ ${result.file} (exit code: ${result.exitCode})`);
    }
  }

  if (results.errors.length > 0) {
    console.log('\n');
    printDelimiter('ERRORS', '=');
    for (const result of results.errors) {
      console.log(`  💥 ${result.file}: ${result.error}`);
    }
  }

  console.log('\n');
  printDelimiter('END OF TEST RUN', '=');

  // Exit with non-zero code if there were any issues
  process.exit(
    results.hung.length > 0 ||
      results.failed.length > 0 ||
      results.errors.length > 0
      ? 1
      : 0,
  );
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
