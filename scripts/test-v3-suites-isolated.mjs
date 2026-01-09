#!/usr/bin/env node
/* eslint-disable no-console */

/**
 * Script to run each v3 integration test suite in isolation and detect hangs.
 *
 * This script:
 * 1. Finds all test files in packages/core/integration-tests/test/
 * 2. Loads state from .test-v3-suites-state.json to track previously passed tests
 * 3. Skips tests that have already passed (based on state file)
 * 4. Runs each remaining test file separately with ATLASPACK_V3=true
 * 5. Monitors for output and detects hangs (30s timeout with no output)
 * 6. Updates state file when tests pass
 * 7. Reports which suites hang or fail
 */

import {spawn} from 'child_process';
import {readdir, stat, readFile, writeFile} from 'fs/promises';
import {join, relative} from 'path';
import {fileURLToPath} from 'url';
import {dirname} from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PROJECT_ROOT = join(__dirname, '..');
const TEST_DIR = join(PROJECT_ROOT, 'packages/core/integration-tests/test');
const STATE_FILE = join(PROJECT_ROOT, '.test-v3-suites-state.json');
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

async function loadState() {
  try {
    const content = await readFile(STATE_FILE, 'utf-8');
    return JSON.parse(content);
  } catch {
    // File doesn't exist or is invalid, return empty state
    return {};
  }
}

async function saveState(state) {
  try {
    await writeFile(STATE_FILE, JSON.stringify(state, null, 2), 'utf-8');
  } catch (error) {
    console.error(`Warning: Failed to save state file: ${error.message}`);
  }
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
          // Pass through hang debug flag if set
          ...(process.env.ATLASPACK_MOCHA_HANG_DEBUG
            ? {
                ATLASPACK_MOCHA_HANG_DEBUG:
                  process.env.ATLASPACK_MOCHA_HANG_DEBUG,
              }
            : {}),
        },
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    );

    let lastOutputTime = Date.now();
    let startTime = Date.now();
    let hasOutput = false;
    let output = '';
    let errorOutput = '';
    let hangDetected = false;
    let testStarted = false; // Track if we've seen Mocha start running tests
    let testsCompleted = false; // Track if we've seen Mocha test completion summary

    const hangTimeout = setInterval(() => {
      // Check if process is still alive
      if (testProcess.killed || testProcess.exitCode !== null) {
        clearInterval(hangTimeout);
        return;
      }

      const timeSinceLastOutput = Date.now() - lastOutputTime;
      const timeSinceStart = Date.now() - startTime;

      // If tests have completed, be very lenient - cleanup/teardown can take time
      // But still have a reasonable timeout (2 minutes for cleanup)
      const cleanupTimeout = 120000; // 2 minutes for cleanup after tests complete
      const hangTimeoutForActiveTests = testStarted ? 180000 : HANG_TIMEOUT_MS; // 3 minutes if tests started

      const effectiveTimeout = testsCompleted
        ? cleanupTimeout
        : hangTimeoutForActiveTests;

      // If we had output before but now it's been silent, check for hang
      if (timeSinceLastOutput >= effectiveTimeout && hasOutput) {
        hangDetected = true;
        testProcess.kill('SIGTERM');
        clearInterval(hangTimeout);
        clearTimeout(maxTimeout);

        // Give it a moment, then force kill if still running
        setTimeout(() => {
          if (!testProcess.killed) {
            testProcess.kill('SIGKILL');
          }
        }, 5000);
      }
      // If we've been running for 60s with no output at all, also consider it a hang
      else if (timeSinceStart >= 60000 && !hasOutput) {
        hangDetected = true;
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
        testProcess.kill('SIGTERM');
        clearInterval(hangTimeout);

        setTimeout(() => {
          if (!testProcess.killed) {
            testProcess.kill('SIGKILL');
          }
        }, 5000);
      }
    }, MAX_TEST_TIMEOUT_MS);

    testProcess.stdout.on('data', (data) => {
      const text = data.toString();
      output += text;
      lastOutputTime = Date.now();
      hasOutput = true;

      // Detect if Mocha has started running tests
      if (
        !testStarted &&
        /(✓|✔|×|✖|passing|failing|pending|tests?)/i.test(text)
      ) {
        testStarted = true;
      }

      // Detect if Mocha has completed all tests (summary output)
      if (!testsCompleted) {
        // Mocha outputs summary like "X passing", "X failing", or shows test counts at the end
        if (
          /(\d+)\s+(passing|failing|pending)/.test(text) ||
          /tests?\s+completed/i.test(text)
        ) {
          testsCompleted = true;
          // Reset timer when we see completion - cleanup can take time
          lastOutputTime = Date.now();
        }
      }
    });

    testProcess.stderr.on('data', (data) => {
      const text = data.toString();
      errorOutput += text;
      lastOutputTime = Date.now();
      hasOutput = true;

      // Detect if Mocha has started running tests (sometimes on stderr)
      if (
        !testStarted &&
        /(✓|✔|×|✖|passing|failing|pending|tests?)/i.test(text)
      ) {
        testStarted = true;
      }

      // Detect if Mocha has completed all tests (summary might be on stderr)
      if (!testsCompleted) {
        if (
          /(\d+)\s+(passing|failing|pending)/.test(text) ||
          /tests?\s+completed/i.test(text)
        ) {
          testsCompleted = true;
          lastOutputTime = Date.now();
        }
      }
    });

    testProcess.on('exit', (code, signal) => {
      clearInterval(hangTimeout);
      clearTimeout(maxTimeout);

      const result = {
        file: relativePath,
        success: code === 0 && !hangDetected,
        hangDetected,
        testsCompleted,
        exitCode: code,
        signal,
        output,
        errorOutput,
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
        testsCompleted: false,
        exitCode: null,
        signal: null,
        output,
        errorOutput: errorOutput + error.message,
        error: error.message,
      });
    });
  });
}

async function main() {
  console.log('Loading test state...');
  const state = await loadState();
  const passedCount = Object.keys(state).filter(
    (key) => state[key] === true,
  ).length;
  console.log(`Found ${passedCount} previously passed test(s) in state file\n`);

  console.log('Finding test files...');
  const testFiles = await findTestFiles(TEST_DIR);
  console.log(`Found ${testFiles.length} test files\n`);

  const results = {
    passed: [],
    skipped: [],
    failed: [],
    hung: [],
    errors: [],
  };

  let runCount = 0;
  for (let i = 0; i < testFiles.length; i++) {
    const testFile = testFiles[i];
    const relativePath = relative(TEST_DIR, testFile);

    // Check if this test has already passed
    if (state[relativePath] === true) {
      results.skipped.push({file: relativePath});
      console.log(
        `[${i + 1}/${testFiles.length}] Skipping (already passed): ${relativePath}`,
      );
      continue;
    }

    runCount++;
    console.log(`[${i + 1}/${testFiles.length}] Running: ${relativePath}`);

    const result = await runTestSuite(testFile);

    if (result.hangDetected) {
      results.hung.push(result);
      const hangTime = result.testsCompleted ? 'cleanup' : 'execution';
      const timeoutUsed = result.testsCompleted ? '120' : '30';
      console.log(
        `  ❌ HANG DETECTED during ${hangTime} (no output for ${timeoutUsed}s+)`,
      );
      if (process.env.ATLASPACK_MOCHA_HANG_DEBUG) {
        console.log(
          `  💡 Tip: Check the output above for open handles (napi_rs_threadsafe_function, FILEHANDLE, etc.)`,
        );
        console.log(
          `  💡 This indicates unresolved async operations preventing process exit`,
        );
      }
      // Remove from state if it was previously marked as passed
      if (state[relativePath] === true) {
        delete state[relativePath];
      }
    } else if (result.success) {
      results.passed.push(result);
      console.log(`  ✅ PASSED`);
      // Update state to mark this test as passed
      state[relativePath] = true;
      await saveState(state);
    } else if (result.error) {
      results.errors.push(result);
      console.log(`  ❌ ERROR: ${result.error}`);
      // Remove from state if it was previously marked as passed
      if (state[relativePath] === true) {
        delete state[relativePath];
      }
    } else {
      results.failed.push(result);
      console.log(`  ❌ FAILED (exit code: ${result.exitCode})`);
      // Remove from state if it was previously marked as passed
      if (state[relativePath] === true) {
        delete state[relativePath];
      }
    }
  }

  console.log('\n' + '='.repeat(80));
  console.log('SUMMARY');
  console.log('='.repeat(80));
  console.log(`Total: ${testFiles.length}`);
  console.log(`Skipped (already passed): ${results.skipped.length}`);
  console.log(`Run: ${runCount}`);
  console.log(`Passed: ${results.passed.length}`);
  console.log(`Failed: ${results.failed.length}`);
  console.log(`Hung: ${results.hung.length}`);
  console.log(`Errors: ${results.errors.length}`);

  if (results.hung.length > 0) {
    console.log('\n' + '='.repeat(80));
    console.log('HANGING TEST SUITES:');
    console.log('='.repeat(80));
    for (const result of results.hung) {
      console.log(`\n${result.file}`);
      console.log('-'.repeat(80));
      if (result.output) {
        console.log('Last output:');
        const lines = result.output.split('\n').slice(-10);
        console.log(lines.join('\n'));
      }
    }
  }

  if (results.failed.length > 0) {
    console.log('\n' + '='.repeat(80));
    console.log('FAILED TEST SUITES:');
    console.log('='.repeat(80));
    for (const result of results.failed) {
      console.log(`\n${result.file} (exit code: ${result.exitCode})`);
    }
  }

  if (results.errors.length > 0) {
    console.log('\n' + '='.repeat(80));
    console.log('ERRORS:');
    console.log('='.repeat(80));
    for (const result of results.errors) {
      console.log(`\n${result.file}: ${result.error}`);
    }
  }

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
