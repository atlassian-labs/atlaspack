/* eslint-disable no-console */
import {performance} from 'node:perf_hooks';
import * as process from 'node:process';
import * as path from 'node:path';
import * as url from 'node:url';
import {execSync} from 'node:child_process';
import {buildFixture} from '../utils/build-fixture.mts';
import {getPhaseMetrics, clearPhaseMetrics} from './PhaseTrackingReporter.mts';
import {
  resetMemoryTracking,
  sampleNativeMemory,
  getNativeMemoryStats,
} from '@atlaspack/rust';
import type {
  BenchmarkOptions,
  BenchmarkSample,
  BenchmarkResult,
  MemorySnapshot,
  DetailedMemoryStats,
  MemoryStats,
  NativeMemoryStats,
  UnifiedMemoryStats,
} from './types.mts';

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function getMemoryUsage(): MemorySnapshot {
  const memory = process.memoryUsage();
  return {
    timestamp: performance.now(),
    heapUsed: memory.heapUsed,
    heapTotal: memory.heapTotal,
    external: memory.external,
    rss: memory.rss,
    arrayBuffers: memory.arrayBuffers,
  };
}

function calculateStats(samples: number[]): DetailedMemoryStats {
  const sorted = [...samples].sort((a, b) => a - b);
  const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
  const variance =
    samples.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) /
    samples.length;

  return {
    min: Math.min(...samples),
    max: Math.max(...samples),
    mean,
    median: sorted[Math.floor(sorted.length / 2)],
    p95: sorted[Math.floor(sorted.length * 0.95)],
    p99: sorted[Math.floor(sorted.length * 0.99)],
    standardDeviation: Math.sqrt(variance),
    range: Math.max(...samples) - Math.min(...samples),
  };
}

function calculateBasicStats(samples: number[]) {
  const sorted = [...samples].sort((a, b) => a - b);
  const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
  const median = sorted[Math.floor(sorted.length / 2)];
  const min = Math.min(...samples);
  const max = Math.max(...samples);

  const variance =
    samples.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) /
    samples.length;
  const standardDeviation = Math.sqrt(variance);

  return {mean, median, min, max, standardDeviation};
}

export async function runBenchmark(
  options: BenchmarkOptions,
): Promise<BenchmarkResult> {
  const {
    samples: sampleCount,
    target,
    name,
    warmupRuns = 1,
    buildOptions,
  } = options;

  console.log(`Running benchmark: ${name}`);
  console.log(`Target: ${target}`);
  console.log(`Samples: ${sampleCount}, Warmup runs: ${warmupRuns}`);

  // Warmup runs
  for (let i = 0; i < warmupRuns; i++) {
    console.log(`Warmup run ${i + 1}/${warmupRuns}`);
    await buildFixture(target, buildOptions);
  }

  const samples: BenchmarkSample[] = [];

  for (let i = 0; i < sampleCount; i++) {
    // Reset native memory tracking between samples
    resetMemoryTracking();

    console.log(`Sample ${i + 1}/${sampleCount}`);

    // Force garbage collection if available
    if (global.gc) {
      global.gc();
    }

    const startTime = performance.now();

    // Sample native memory during the benchmark at regular intervals
    const memoryInterval = setInterval(() => {
      // Sample native memory during the benchmark
      sampleNativeMemory();
    }, 50); // Sample every 50ms

    try {
      // Clear any previous phase tracking data
      clearPhaseMetrics(`sample_${i}`);

      // Try to run with phase tracking reporter if possible

      // Set the benchmark instance ID in environment for the reporter to pick up
      process.env.BENCHMARK_INSTANCE_ID = `sample_${i}`;

      // Attempt to use phase tracking reporter
      await buildFixture(target, {
        ...buildOptions,
        additionalReporters: [
          {
            packageName: path.join(__dirname, 'PhaseTrackingReporter.mts'),
            resolveFrom: __dirname,
          },
        ],
      });

      // Get detailed phase metrics if available - try different possible instance IDs
      let phaseMetrics = getPhaseMetrics(`sample_${i}`);

      if (phaseMetrics && phaseMetrics.phases.length > 0) {
        // Use detailed phase metrics
        const endTime = performance.now();
        const endMemory = getMemoryUsage();

        clearInterval(memoryInterval);

        const sample: BenchmarkSample = {
          totalDuration: endTime - startTime,
          phases: phaseMetrics.phases,
          memoryPeak: phaseMetrics.globalMemoryPeak,
          memoryEnd: endMemory,
          overallMemoryStats: phaseMetrics.memoryStats,
          globalMemoryPeak: phaseMetrics.globalMemoryPeak,
          nativeMemoryStats: undefined,
        };

        samples.push(sample);
      } else {
        throw new Error(`Metrics for sample_${i} was not found`);
      }
    } catch (error) {
      clearInterval(memoryInterval);
      console.error(`Sample ${i + 1} failed:`, error);
      throw error;
    } finally {
      // Clean up environment variable
      delete process.env.BENCHMARK_INSTANCE_ID;
    }
  }

  // Calculate comprehensive statistics
  const durations = samples.map((s) => s.totalDuration);
  const memoryPeaks = samples.map((s) => s.memoryPeak.rss);

  const durationStats = calculateBasicStats(durations);
  const memoryStats = calculateBasicStats(memoryPeaks);

  // Calculate phase statistics
  const phaseStats: {
    [phaseName: string]: {
      duration: DetailedMemoryStats;
      memoryPeak: DetailedMemoryStats;
    };
  } = {};

  // Collect all unique phase names
  const phaseNames = new Set<string>();
  samples.forEach((sample) => {
    sample.phases.forEach((phase) => phaseNames.add(phase.name));
  });

  // Calculate stats for each phase
  phaseNames.forEach((phaseName) => {
    const phaseDurations: number[] = [];
    const phaseMemoryPeaks: number[] = [];

    samples.forEach((sample) => {
      const phase = sample.phases.find((p) => p.name === phaseName);
      if (phase) {
        phaseDurations.push(phase.duration);
        phaseMemoryPeaks.push(phase.memoryPeak.rss);
      }
    });

    if (phaseDurations.length > 0) {
      phaseStats[phaseName] = {
        duration: calculateStats(phaseDurations),
        memoryPeak: calculateStats(phaseMemoryPeaks),
      };
    }
  });

  // Calculate overall memory statistics from all samples
  let overallMemoryStats: MemoryStats | null = null;
  const allMemorySnapshots = samples.flatMap((s) =>
    s.overallMemoryStats ? [s.memoryPeak, s.memoryEnd] : [],
  );

  if (allMemorySnapshots.length > 0) {
    const rssValues = allMemorySnapshots.map((s) => s.rss);
    const heapUsedValues = allMemorySnapshots.map((s) => s.heapUsed);
    const heapTotalValues = allMemorySnapshots.map((s) => s.heapTotal);
    const externalValues = allMemorySnapshots.map((s) => s.external);

    overallMemoryStats = {
      rss: calculateStats(rssValues),
      heapUsed: calculateStats(heapUsedValues),
      heapTotal: calculateStats(heapTotalValues),
      external: calculateStats(externalValues),
      sampleCount: allMemorySnapshots.length,
    };
  }

  // Calculate unified memory statistics (JS + Native)
  let unifiedMemoryStats: UnifiedMemoryStats | null = null;

  // Try to get overall native memory stats from the samples collected during benchmarks
  let nativeMemoryStats: NativeMemoryStats | null = getNativeMemoryStats();

  if (overallMemoryStats) {
    unifiedMemoryStats = {
      js: overallMemoryStats,
      native: nativeMemoryStats || undefined,
    };
  }

  // Get git information
  const gitSha = execSync('git rev-parse HEAD', {encoding: 'utf8'}).trim();
  const gitBranch = execSync('git rev-parse --abbrev-ref HEAD', {
    encoding: 'utf8',
  }).trim();

  return {
    name,
    target,
    samples,
    stats: {
      mean: durationStats.mean,
      median: durationStats.median,
      min: durationStats.min,
      max: durationStats.max,
      standardDeviation: durationStats.standardDeviation,
      memoryPeakMean: memoryStats.mean,
      memoryPeakMedian: memoryStats.median,
      phaseStats,
      overallMemoryStats,
      unifiedMemoryStats,
    },
    timestamp: new Date().toISOString(),
    gitSha,
    gitBranch,
  };
}
