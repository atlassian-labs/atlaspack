#!/usr/bin/env node
/* eslint-disable no-console */

/**
 * CI script to run integration tests with slicing support.
 *
 * This script:
 * 1. Finds all test files in packages/core/integration-tests/test/
 * 2. Splits tests into slices if SLICE_NUM and SLICE_TOTAL are provided
 * 3. Runs all test files in the slice together using test-ci
 * 4. Shows clear delimiters to identify which slice is running
 * 5. Passes through ATLASPACK_V3 if set (for v3 tests)
 *
 * Usage:
 *   # Run all tests (non-v3)
 *   node scripts/test-integration-sliced-ci.mjs
 *
 *   # Run slice 1/2 (non-v3)
 *   SLICE_NUM=1 SLICE_TOTAL=2 node scripts/test-integration-sliced-ci.mjs
 *
 *   # Run all tests (v3)
 *   ATLASPACK_V3=true node scripts/test-integration-sliced-ci.mjs
 *
 *   # Run slice 1/2 (v3)
 *   ATLASPACK_V3=true SLICE_NUM=1 SLICE_TOTAL=2 node scripts/test-integration-sliced-ci.mjs
 */

import {spawn} from 'child_process';
import {join} from 'path';
import {fileURLToPath} from 'url';
import {dirname} from 'path';
import fastGlob from 'fast-glob';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PROJECT_ROOT = join(__dirname, '..');
const TEST_DIR = join(PROJECT_ROOT, 'packages/core/integration-tests/test');

async function findTestFiles() {
  const files = await fastGlob('*.ts', {
    cwd: TEST_DIR,
    absolute: true,
    onlyFiles: true,
  });

  return files.sort();
}

function getSlice(testFiles, sliceNum, sliceTotal) {
  if (!sliceNum || !sliceTotal) {
    return testFiles;
  }

  const sliceNumInt = parseInt(sliceNum, 10);
  const sliceTotalInt = parseInt(sliceTotal, 10);

  if (
    isNaN(sliceNumInt) ||
    isNaN(sliceTotalInt) ||
    sliceNumInt < 1 ||
    sliceNumInt > sliceTotalInt
  ) {
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

function runTests(testFiles) {
  return new Promise((resolve) => {
    const testProcess = spawn(
      'yarn',
      ['workspace', '@atlaspack/integration-tests', 'test-ci', ...testFiles],
      {
        cwd: PROJECT_ROOT,
        stdio: 'inherit',
      },
    );

    testProcess.on('exit', (code) => {
      resolve({success: code === 0, exitCode: code});
    });

    testProcess.on('error', (error) => {
      resolve({success: false, exitCode: null, error: error.message});
    });
  });
}

async function main() {
  const sliceNum = process.env.SLICE_NUM;
  const sliceTotal = process.env.SLICE_TOTAL;
  const isV3 = process.env.ATLASPACK_V3 === 'true';
  const testType = isV3 ? 'V3 INTEGRATION TESTS' : 'INTEGRATION TESTS';

  console.log('\n');
  if (sliceNum && sliceTotal) {
    printDelimiter(`${testType} - SLICE ${sliceNum}/${sliceTotal}`, '=');
  } else {
    printDelimiter(testType, '=');
  }

  console.log('Finding test files...');
  const allTestFiles = await findTestFiles();
  console.log(`Found ${allTestFiles.length} test files`);

  const testFiles = getSlice(allTestFiles, sliceNum, sliceTotal);

  if (sliceNum && sliceTotal) {
    console.log(
      `Running slice ${sliceNum}/${sliceTotal}: ${testFiles.length} test files\n`,
    );
  } else {
    console.log(`Running all ${testFiles.length} test files\n`);
  }

  console.log('\n');
  if (sliceNum && sliceTotal) {
    printDelimiter(
      `SLICE ${sliceNum}/${sliceTotal}: Running ${testFiles.length} test files`,
      '=',
    );
  } else {
    printDelimiter(`Running ${testFiles.length} test files`, '=');
  }
  console.log('');

  const result = await runTests(testFiles);

  console.log('');
  if (result.success) {
    if (sliceNum && sliceTotal) {
      printDelimiter(`✅ PASSED: Slice ${sliceNum}/${sliceTotal}`, '-');
    } else {
      printDelimiter('✅ PASSED: All tests', '-');
    }
  } else if (result.error) {
    if (sliceNum && sliceTotal) {
      printDelimiter(
        `❌ ERROR: Slice ${sliceNum}/${sliceTotal} - ${result.error}`,
        '-',
      );
    } else {
      printDelimiter(`❌ ERROR: ${result.error}`, '-');
    }
  } else {
    if (sliceNum && sliceTotal) {
      printDelimiter(
        `❌ FAILED: Slice ${sliceNum}/${sliceTotal} (exit code: ${result.exitCode})`,
        '-',
      );
    } else {
      printDelimiter(
        `❌ FAILED: All tests (exit code: ${result.exitCode})`,
        '-',
      );
    }
  }

  console.log('\n');
  printDelimiter('END OF TEST RUN', '=');

  process.exitCode = result.success ? 0 : 1;
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exitCode = 1;
});
