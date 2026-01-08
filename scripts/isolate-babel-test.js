#!/usr/bin/env node

/* eslint-disable no-console */
const fs = require('fs');
const path = require('path');
const {spawn} = require('child_process');

const testFile = path.join(
  __dirname,
  '../packages/core/integration-tests/test/babel.ts',
);

function findTestIndices(content) {
  // Match different it() patterns - need to check in order of specificity
  const patterns = [
    {regex: /\bit\.skip\(/g, type: 'skip'},
    {regex: /\bit\.v2\.only\(/g, type: 'v2only'},
    {regex: /\bit\.only\(/g, type: 'only'},
    {regex: /\bit\.v2\(/g, type: 'v2'},
    {regex: /\bit\(/g, type: 'it'},
  ];

  const allMatches = [];

  for (const pattern of patterns) {
    let match;
    pattern.regex.lastIndex = 0;
    while ((match = pattern.regex.exec(content)) !== null) {
      allMatches.push({
        index: match.index,
        type: pattern.type,
        match: match[0],
      });
    }
  }

  // Sort by index and remove duplicates (prefer more specific matches)
  allMatches.sort((a, b) => a.index - b.index);

  // Remove overlapping matches (keep the first/longest one)
  const filtered = [];
  for (let i = 0; i < allMatches.length; i++) {
    const current = allMatches[i];
    const prev = filtered[filtered.length - 1];

    if (!prev || current.index >= prev.index + prev.match.length) {
      // Only include non-skip tests
      if (current.type !== 'skip') {
        filtered.push(current);
      }
    }
  }

  return filtered;
}

function addOnlyToTest(content, testIndex) {
  // First, remove all .only modifiers
  let newContent = content
    .replace(/it\.only\(/g, 'it(')
    .replace(/it\.v2\.only\(/g, 'it.v2(');

  // Find all tests again after cleanup
  const tests = findTestIndices(newContent);

  if (testIndex >= tests.length) {
    return newContent;
  }

  const targetTest = tests[testIndex];
  const start = targetTest.index;
  const original = targetTest.match;

  // Determine replacement based on type
  let replacement;
  if (targetTest.type === 'v2') {
    replacement = 'it.v2.only(';
  } else if (targetTest.type === 'it') {
    replacement = 'it.only(';
  } else {
    // Already has .only or unexpected type
    return newContent;
  }

  newContent =
    newContent.substring(0, start) +
    replacement +
    newContent.substring(start + original.length);

  return newContent;
}

function runTest(testIndex, testName) {
  return new Promise((resolve) => {
    console.log(`\n${'='.repeat(80)}`);
    console.log(`Running test ${testIndex + 1}: ${testName}`);
    console.log('='.repeat(80));

    const env = {
      ...process.env,
      ATLASPACK_TRACING_MODE: 'stdout',
      RUST_LOG: 'error',
      ATLASPACK_MOCHA_HANG_DEBUG: 'true',
      ATLASPACK_V3: 'true',
    };

    const testProcess = spawn('yarn', ['test:integration', 'test/babel.ts'], {
      env,
      stdio: 'inherit',
      shell: true,
      cwd: path.join(__dirname, '..'),
    });

    testProcess.on('close', (code) => {
      resolve(code);
    });

    testProcess.on('error', (err) => {
      console.error(`Failed to start test process: ${err.message}`);
      resolve(1);
    });
  });
}

async function main() {
  const startIndex = parseInt(process.argv[2] || '0', 10);
  const stopOnFailure =
    process.argv.includes('--stop-on-failure') || process.argv.includes('-s');

  if (!fs.existsSync(testFile)) {
    console.error(`Test file not found: ${testFile}`);
    process.exit(1);
  }

  let originalContent = fs.readFileSync(testFile, 'utf8');
  const tests = findTestIndices(originalContent);

  if (tests.length === 0) {
    console.log('No tests found');
    return;
  }

  console.log(`\nFound ${tests.length} tests to process`);
  console.log(`Starting from test ${startIndex + 1}`);
  if (stopOnFailure) {
    console.log('Will stop on first failure');
  }
  console.log('');

  const results = [];
  let failedTests = [];

  // Process each test
  for (let i = startIndex; i < tests.length; i++) {
    const test = tests[i];

    // Extract test name from original content
    const afterMatch = originalContent.substring(test.index);
    const testNameMatch = afterMatch.match(/['"`]([^'"`]+)['"`]/);
    const testName = testNameMatch ? testNameMatch[1] : `Test ${i + 1}`;

    // Add .only to current test (this will clean up any existing .only first)
    let content = addOnlyToTest(originalContent, i);
    fs.writeFileSync(testFile, content, 'utf8');

    // Run the test
    const exitCode = await runTest(i, testName);
    const passed = exitCode === 0;

    results.push({
      index: i + 1,
      name: testName,
      passed,
      exitCode,
    });

    if (!passed) {
      console.log(`\n❌ Test ${i + 1} FAILED: ${testName}`);
      failedTests.push({index: i + 1, name: testName});

      if (stopOnFailure) {
        console.log(
          '\nStopping on first failure (use without --stop-on-failure to continue)',
        );
        break;
      }
    } else {
      console.log(`\n✓ Test ${i + 1} PASSED: ${testName}`);
    }
  }

  // Clean up: remove all .only modifiers
  console.log('\n' + '='.repeat(80));
  console.log('Cleaning up: Removing all .only modifiers...');
  let finalContent = originalContent
    .replace(/it\.only\(/g, 'it(')
    .replace(/it\.v2\.only\(/g, 'it.v2(');
  fs.writeFileSync(testFile, finalContent, 'utf8');
  console.log('✓ Cleanup complete');

  // Print summary
  console.log('\n' + '='.repeat(80));
  console.log('SUMMARY');
  console.log('='.repeat(80));
  console.log(`Total tests run: ${results.length}`);
  console.log(`Passed: ${results.filter((r) => r.passed).length}`);
  console.log(`Failed: ${failedTests.length}`);

  if (failedTests.length > 0) {
    console.log('\nFailed tests:');
    failedTests.forEach(({index, name}) => {
      console.log(`  ${index}. ${name}`);
    });
  }

  console.log('='.repeat(80));

  // Exit with error code if any tests failed
  process.exit(failedTests.length > 0 ? 1 : 0);
}

main();
