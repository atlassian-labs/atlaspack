#!/usr/bin/env node --experimental-strip-types
/* eslint-disable no-console */
import {mkdir} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import * as path from 'node:path';
import {runBenchmark} from './benchmarks/runner.mts';
import {
  createReport,
  saveReport,
  saveTextReport,
  loadBaselineReport,
  compareResults,
  generateMarkdownReport,
  generateGitHubComment,
} from './benchmarks/reporter.mts';
import {
  BENCHMARK_CONFIGS,
  DEFAULT_OUTPUT_DIR,
  BASELINE_FILENAME,
  CURRENT_FILENAME,
} from './benchmarks/config.mts';
import type {BenchmarkReport, BenchmarkResult} from './benchmarks/types.mts';

async function main() {
  const args = process.argv.slice(2);
  const outputDir =
    args.find((arg) => arg.startsWith('--output='))?.split('=')[1] ||
    DEFAULT_OUTPUT_DIR;
  const baselinePath = args
    .find((arg) => arg.startsWith('--baseline='))
    ?.split('=')[1];
  const generateComment = args.includes('--github-comment');
  const specificTest = args
    .find((arg) => arg.startsWith('--test='))
    ?.split('=')[1];
  const samples = parseInt(
    args.find((arg) => arg.startsWith('--samples='))?.split('=')[1] || '5',
  );

  console.log('🚀 Starting Atlaspack benchmarks...\n');
  console.log(`Debug: outputDir = ${outputDir}`);
  console.log(`Debug: generateComment = ${generateComment}`);
  console.log(`Debug: args = ${JSON.stringify(args)}`);

  // Ensure output directory exists
  if (!existsSync(outputDir)) {
    await mkdir(outputDir, {recursive: true});
  }

  // Filter benchmarks if specific test requested
  let benchmarksToRun = BENCHMARK_CONFIGS;
  if (specificTest) {
    benchmarksToRun = BENCHMARK_CONFIGS.filter(
      (config) =>
        config.name.toLowerCase().includes(specificTest.toLowerCase()) ||
        config.target.includes(specificTest),
    );

    if (benchmarksToRun.length === 0) {
      console.error(`❌ No benchmark found matching: ${specificTest}`);
      console.log('\nAvailable benchmarks:');
      BENCHMARK_CONFIGS.forEach((config) => {
        console.log(`  - ${config.name} (${config.target})`);
      });
      process.exit(1);
    }
  }

  // Override sample count if specified
  if (samples !== 5) {
    benchmarksToRun = benchmarksToRun.map((config) => ({
      ...config,
      samples,
    }));
  }

  console.log(
    `Running ${benchmarksToRun.length} benchmark(s) with ${samples} samples each...\n`,
  );

  // Run benchmarks
  const results: BenchmarkResult[] = [];
  for (const config of benchmarksToRun) {
    try {
      const result = await runBenchmark(config);
      results.push(result);
      console.log(`✅ Completed: ${config.name}\n`);
    } catch (error) {
      console.error(`❌ Failed: ${config.name}`);
      console.error(error);
      process.exit(1);
    }
  }

  // Create and save current report
  const currentReport = createReport(results);
  const currentReportPath = path.join(outputDir, CURRENT_FILENAME);
  await saveReport(currentReport, currentReportPath);

  // Load baseline for comparison if available
  let baselineReport: BenchmarkReport | null = null;
  if (baselinePath) {
    baselineReport = await loadBaselineReport(baselinePath);
  } else {
    // Try to load baseline from output directory
    const defaultBaselinePath = path.join(outputDir, BASELINE_FILENAME);
    baselineReport = await loadBaselineReport(defaultBaselinePath);
  }

  // Compare results
  const comparisons = results.map((result) => {
    const baselineResult = baselineReport?.results.find(
      (r) => r.name === result.name,
    );
    return compareResults(result, baselineResult);
  });

  // Generate reports
  console.log('\n📊 Benchmark Results:\n');
  const markdownReport = generateMarkdownReport(comparisons);
  console.log(markdownReport);

  // Save markdown report
  const markdownPath = path.join(outputDir, 'report.md');
  await saveTextReport(markdownReport, markdownPath);

  // Generate GitHub comment if requested
  console.log(`Debug: About to check generateComment: ${generateComment}`);
  if (generateComment) {
    console.log('Debug: Generating GitHub comment...');
    const commentPath = path.join(outputDir, 'github-comment.md');
    console.log(`Debug: commentPath = ${commentPath}`);
    const comment = await generateGitHubComment(comparisons);
    console.log(`Debug: comment length = ${comment.length}`);
    await saveTextReport(comment, commentPath);
    console.log(`\n💬 GitHub comment saved to: ${commentPath}`);
  } else {
    console.log('Debug: GitHub comment generation skipped - generateComment is false');
  }

  // Check for regressions
  const regressions = comparisons.filter((c) => c.comparison?.isRegression);
  if (regressions.length > 0) {
    console.log(
      `\n⚠️  ${regressions.length} performance regression(s) detected!`,
    );
    regressions.forEach((r) => {
      console.log(
        `   - ${r.name}: ${r.comparison?.meanDiffPercent?.toFixed(2)}% slower`,
      );
    });

    // Exit with error code if regressions detected
    if (process.env.CI === 'true') {
      process.exit(1);
    }
  }

  const improvements = comparisons.filter((c) => c.comparison?.isImprovement);
  if (improvements.length > 0) {
    console.log(
      `\n🎉 ${improvements.length} performance improvement(s) detected!`,
    );
    improvements.forEach((i) => {
      console.log(
        `   - ${i.name}: ${Math.abs(i.comparison?.meanDiffPercent || 0).toFixed(2)}% faster`,
      );
    });
  }

  console.log('\n✨ Benchmarking complete!');
}

// Handle errors
process.on('unhandledRejection', (error) => {
  console.error('❌ Unhandled rejection:', error);
  process.exit(1);
});

process.on('uncaughtException', (error) => {
  console.error('❌ Uncaught exception:', error);
  process.exit(1);
});

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error('❌ Benchmark failed:', error);
    process.exit(1);
  });
}
