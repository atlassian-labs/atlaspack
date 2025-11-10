import {InitialAtlaspackOptions} from '@atlaspack/types';

export interface BenchmarkOptions {
  samples: number;
  target: string;
  name: string;
  warmupRuns?: number;
  buildOptions?: InitialAtlaspackOptions;
}

export interface NativeMemoryStatsSimple {
  totalAllocated: number;
  peakAllocated: number;
  activeAllocations: number;
}

export interface MemorySnapshot {
  timestamp: number;
  heapUsed: number;
  heapTotal: number;
  external: number;
  rss: number;
  arrayBuffers: number;
  nativeMemory?: NativeMemoryStatsSimple;
}

export interface DetailedMemoryStats {
  min: number;
  max: number;
  mean: number;
  median: number;
  p95: number;
  p99: number;
  standardDeviation: number;
  range: number;
}

export interface MemoryStats {
  rss: DetailedMemoryStats;
  heapUsed: DetailedMemoryStats;
  heapTotal: DetailedMemoryStats;
  external: DetailedMemoryStats;
  sampleCount: number;
}

export interface NativeMemoryStats {
  physicalMem: DetailedMemoryStats;
  virtualMem: DetailedMemoryStats;
  sampleCount: number;
}

export interface UnifiedMemoryStats {
  js: MemoryStats;
  native?: NativeMemoryStats;
}

export interface PhaseMetrics {
  name: string;
  duration: number;
  startTime: number;
  endTime: number;
  startMemory: MemorySnapshot;
  endMemory: MemorySnapshot;
  memoryPeak: MemorySnapshot;
  memorySnapshots: number;
  memoryStats: MemoryStats | null;
  gcCount: number;
  gcTime: number;
}

export interface BenchmarkSample {
  totalDuration: number;
  phases: PhaseMetrics[];
  memoryPeak: MemorySnapshot;
  memoryEnd: MemorySnapshot;
  overallMemoryStats: MemoryStats | null;
  globalMemoryPeak: MemorySnapshot;
  nativeMemoryStats?: NativeMemoryStatsSimple;
}

export interface BenchmarkResult {
  name: string;
  target: string;
  samples: BenchmarkSample[];
  stats: {
    mean: number;
    median: number;
    min: number;
    max: number;
    standardDeviation: number;
    memoryPeakMean: number;
    memoryPeakMedian: number;
    phaseStats: {
      [phaseName: string]: {
        duration: DetailedMemoryStats;
        memoryPeak: DetailedMemoryStats;
      };
    };
    overallMemoryStats: MemoryStats | null;
    unifiedMemoryStats: UnifiedMemoryStats | null;
  };
  timestamp: string;
  gitSha: string;
  gitBranch: string;
}

export interface BenchmarkReport {
  results: BenchmarkResult[];
  environment: {
    nodeVersion: string;
    platform: string;
    arch: string;
    cpu: string;
    memory: number;
  };
  timestamp: string;
}

export interface ComparisonResult {
  name: string;
  current: BenchmarkResult;
  baseline?: BenchmarkResult;
  comparison?: {
    meanDiff: number;
    meanDiffPercent: number;
    memoryDiff: number;
    memoryDiffPercent: number;
    isRegression: boolean;
    isImprovement: boolean;
  };
}
