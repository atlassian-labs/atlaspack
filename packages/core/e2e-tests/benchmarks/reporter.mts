/* eslint-disable no-console */
import {writeFile, readFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import os from 'node:os';
import type {
  BenchmarkResult,
  BenchmarkReport,
  ComparisonResult,
} from './types.mts';
import { MEMORY_THRESHOLD, PERFORMANCE_THRESHOLD } from './config.mts';

function getCpuInfo(): string {
  const cpus = os.cpus();
  return cpus[0]?.model || 'Unknown CPU';
}

export function createReport(results: BenchmarkResult[]): BenchmarkReport {
  return {
    results,
    environment: {
      nodeVersion: process.version,
      platform: process.platform,
      arch: process.arch,
      cpu: getCpuInfo(),
      memory: os.totalmem(),
    },
    timestamp: new Date().toISOString(),
  };
}

export async function saveReport(report: BenchmarkReport, outputPath: string): Promise<void> {
  await writeFile(outputPath, JSON.stringify(report, null, 2));
  console.log(`Benchmark report saved to: ${outputPath}`);
}

export async function saveTextReport(content: string, outputPath: string): Promise<void> {
  await writeFile(outputPath, content);
  console.log(`Benchmark report saved to: ${outputPath}`);
}

export async function loadBaselineReport(baselinePath: string): Promise<BenchmarkReport | null> {
  if (!existsSync(baselinePath)) {
    console.warn(`Baseline report not found at: ${baselinePath}`);
    return null;
  }

  try {
    const content = await readFile(baselinePath, 'utf8');
    return JSON.parse(content);
  } catch (error) {
    console.error(`Failed to load baseline report: ${error}`);
    return null;
  }
}

export function compareResults(
  current: BenchmarkResult,
  baseline: BenchmarkResult | undefined,
): ComparisonResult {
  if (!baseline) {
    return {
      name: current.name,
      current,
    };
  }

  const meanDiff = current.stats.mean - baseline.stats.mean;
  const meanDiffPercent = (meanDiff / baseline.stats.mean) * 100;

  const memoryDiff = current.stats.memoryPeakMean - baseline.stats.memoryPeakMean;
  const memoryDiffPercent = (memoryDiff / baseline.stats.memoryPeakMean) * 100;

  const isRegression = meanDiffPercent > PERFORMANCE_THRESHOLD || memoryDiffPercent > MEMORY_THRESHOLD;
  const isImprovement = meanDiffPercent < -PERFORMANCE_THRESHOLD || memoryDiffPercent < -MEMORY_THRESHOLD;

  return {
    name: current.name,
    current,
    baseline,
    comparison: {
      meanDiff,
      meanDiffPercent,
      memoryDiff,
      memoryDiffPercent,
      isRegression,
      isImprovement,
    },
  };
}

export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms.toFixed(2)}ms`;
  }
  return `${(ms / 1000).toFixed(2)}s`;
}

export function formatMemory(bytes: number): string {
  const mb = bytes / 1024 / 1024;
  if (mb < 1024) {
    return `${mb.toFixed(2)}MB`;
  }
  const gb = mb / 1024;
  return `${gb.toFixed(2)}GB`;
}

export function formatPercentage(percent: number): string {
  const sign = percent > 0 ? '+' : '';
  return `${sign}${percent.toFixed(2)}%`;
}

export function generateMarkdownReport(comparisons: ComparisonResult[]): string {
  let markdown = '# 📊 Benchmark Results\n\n';

  if (comparisons.length === 0) {
    return markdown + 'No benchmark results to display.\n';
  }

  // Overall results table
  markdown += '## Overall Performance\n\n';
  markdown += '| Test | Duration | Memory Peak | vs Baseline | Status |\n';
  markdown += '|------|----------|-------------|-------------|--------|\n';

  for (const comparison of comparisons) {
    const {name, current, comparison: comp} = comparison;

    const duration = formatDuration(current.stats.mean);
    const memory = formatMemory(current.stats.memoryPeakMean);

    let vsBaseline = 'N/A';
    let status = '⚪ New';

    if (comp) {
      const durationChange = formatPercentage(comp.meanDiffPercent);
      const memoryChange = formatPercentage(comp.memoryDiffPercent);
      vsBaseline = `${durationChange} duration, ${memoryChange} memory`;

      if (comp.isRegression) {
        status = '🔴 Regression';
      } else if (comp.isImprovement) {
        status = '🟢 Improvement';
      } else {
        status = '🟡 Neutral';
      }
    }

    markdown += `| ${name} | ${duration} | ${memory} | ${vsBaseline} | ${status} |\n`;
  }

  // Detailed phase analysis
  markdown += '\n## 🔍 Detailed Phase Analysis\n\n';
  for (const comparison of comparisons) {
    const {name, current} = comparison;

    if (Object.keys(current.stats.phaseStats).length > 0) {
      markdown += `### ${name}\n\n`;
      markdown += '| Phase | Duration (avg) | Duration (p95) | Memory Peak (avg) | Memory Peak (p95) |\n';
      markdown += '|-------|----------------|----------------|-------------------|-------------------|\n';

      Object.entries(current.stats.phaseStats).forEach(([phaseName, stats]) => {
        const avgDuration = formatDuration(stats.duration.mean);
        const p95Duration = formatDuration(stats.duration.p95);
        const avgMemory = formatMemory(stats.memoryPeak.mean);
        const p95Memory = formatMemory(stats.memoryPeak.p95);

        markdown += `| ${phaseName} | ${avgDuration} | ${p95Duration} | ${avgMemory} | ${p95Memory} |\n`;
      });

      markdown += '\n';
    }
  }

  // Memory statistics
  markdown += '\n## 💾 Memory Analysis\n\n';
  for (const comparison of comparisons) {
    const {name, current} = comparison;

    if (current.stats.overallMemoryStats) {
      markdown += `### ${name} Memory Statistics\n\n`;

      const memStats = current.stats.overallMemoryStats;

      markdown += '| Metric | Min | Mean | Median | P95 | P99 | Max | Std Dev |\n';
      markdown += '|--------|-----|------|--------|-----|-----|-----|----------|\n';

      const formatMemStat = (stat: any) => `${formatMemory(stat.min)} | ${formatMemory(stat.mean)} | ${formatMemory(stat.median)} | ${formatMemory(stat.p95)} | ${formatMemory(stat.p99)} | ${formatMemory(stat.max)} | ${formatMemory(stat.standardDeviation)}`;

      markdown += `| RSS | ${formatMemStat(memStats.rss)} |\n`;
      markdown += `| Heap Used | ${formatMemStat(memStats.heapUsed)} |\n`;
      markdown += `| Heap Total | ${formatMemStat(memStats.heapTotal)} |\n`;
      markdown += `| External | ${formatMemStat(memStats.external)} |\n`;

      markdown += `\n**Sample Count**: ${memStats.sampleCount} memory measurements\n\n`;
    }
  }

  // Environment information
  markdown += '\n## 🖥️ Environment\n\n';
  const env = comparisons[0]?.current ? comparisons[0].current : null;
  if (env) {
    markdown += `- **Node.js**: ${process.version}\n`;
    markdown += `- **Platform**: ${process.platform} (${process.arch})\n`;
    markdown += `- **CPU**: ${getCpuInfo()}\n`;
    markdown += `- **Total Memory**: ${formatMemory(os.totalmem())}\n`;
    markdown += `- **Git SHA**: ${env.gitSha.substring(0, 8)}\n`;
    markdown += `- **Branch**: ${env.gitBranch}\n`;
    markdown += `- **Timestamp**: ${env.timestamp}\n`;
  }

  return markdown;
}

export function generateGitHubComment(comparisons: ComparisonResult[]): string {
  const hasRegressions = comparisons.some(c => c.comparison?.isRegression);
  const hasImprovements = comparisons.some(c => c.comparison?.isImprovement);

  let header = '## 📊 Benchmark Results\n\n';

  if (hasRegressions) {
    header += '⚠️ **Performance regressions detected!**\n\n';
  } else if (hasImprovements) {
    header += '🎉 **Performance improvements detected!**\n\n';
  } else {
    header += '✅ **No significant performance changes detected.**\n\n';
  }

  return header + generateMarkdownReport(comparisons);
}
