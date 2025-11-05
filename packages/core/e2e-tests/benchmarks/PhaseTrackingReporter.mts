import {Reporter} from '@atlaspack/plugin';
import {performance} from 'node:perf_hooks';
import type {MemorySnapshot, PhaseMetrics} from './types.mts';

let getNativeMemoryStats: undefined | (() => any) | null;

// Lazy load to avoid top-level await
function ensureNativeMemoryStats() {
  if (getNativeMemoryStats === undefined) {
    try {
      // eslint-disable-next-line @typescript-eslint/no-var-requires
      const atlaspackRust = require('@atlaspack/rust');
      getNativeMemoryStats = atlaspackRust.getNativeMemoryStats || null;
    } catch {
      getNativeMemoryStats = null;
    }
  }
  return getNativeMemoryStats;
}

interface PhaseTracker {
  name: string;
  startTime: number;
  startMemory: MemorySnapshot;
  memorySnapshots: MemorySnapshot[];
  memoryPeak: MemorySnapshot;
}

interface BuildMetrics {
  instanceId: string;
  phases: Map<string, PhaseTracker>;
  currentPhase: string | null;
  overallStartTime: number;
  memorySnapshots: MemorySnapshot[];
  globalMemoryPeak: MemorySnapshot;
  memoryInterval: NodeJS.Timeout | null;
}

// Global storage for build metrics
const buildMetrics = new Map<string, BuildMetrics>();

function takeMemorySnapshot(): MemorySnapshot {
  const memory = process.memoryUsage();
  let nativeMemory;
  try {
    const fn = ensureNativeMemoryStats();
    nativeMemory = fn ? fn() : undefined;
  } catch {
    // Native memory stats not available
  }
  return {
    timestamp: performance.now(),
    heapUsed: memory.heapUsed,
    heapTotal: memory.heapTotal,
    external: memory.external,
    rss: memory.rss,
    arrayBuffers: memory.arrayBuffers,
    nativeMemory,
  };
}

function updateMemoryPeak(
  current: MemorySnapshot,
  peak: MemorySnapshot,
): MemorySnapshot {
  return current.rss > peak.rss ? current : peak;
}

function startMemoryTracking(metrics: BuildMetrics) {
  if (metrics.memoryInterval) {
    clearInterval(metrics.memoryInterval);
  }

  // High-frequency memory sampling during build
  metrics.memoryInterval = setInterval(() => {
    const snapshot = takeMemorySnapshot();
    metrics.memorySnapshots.push(snapshot);
    metrics.globalMemoryPeak = updateMemoryPeak(
      snapshot,
      metrics.globalMemoryPeak,
    );

    // Update current phase memory tracking
    if (metrics.currentPhase) {
      const phase = metrics.phases.get(metrics.currentPhase);
      if (phase) {
        phase.memorySnapshots.push(snapshot);
        phase.memoryPeak = updateMemoryPeak(snapshot, phase.memoryPeak);
      }
    }
  }, 25); // Sample every 25ms for very high resolution
}

function stopMemoryTracking(metrics: BuildMetrics) {
  if (metrics.memoryInterval) {
    clearInterval(metrics.memoryInterval);
    metrics.memoryInterval = null;
  }
}

function initializeBuildMetrics(instanceId: string): BuildMetrics {
  const initialMemory = takeMemorySnapshot();
  const metrics: BuildMetrics = {
    instanceId,
    phases: new Map(),
    currentPhase: null,
    overallStartTime: performance.now(),
    memorySnapshots: [initialMemory],
    globalMemoryPeak: initialMemory,
    memoryInterval: null,
  };

  buildMetrics.set(instanceId, metrics);
  startMemoryTracking(metrics);
  return metrics;
}

function startPhase(metrics: BuildMetrics, phaseName: string) {
  // Finalize previous phase if exists
  if (metrics.currentPhase) {
    finalizePhase(metrics, metrics.currentPhase);
  }

  const startTime = performance.now();
  const startMemory = takeMemorySnapshot();

  const phase: PhaseTracker = {
    name: phaseName,
    startTime,
    startMemory,
    memorySnapshots: [startMemory],
    memoryPeak: startMemory,
  };

  metrics.phases.set(phaseName, phase);
  metrics.currentPhase = phaseName;
}

function finalizePhase(metrics: BuildMetrics, phaseName: string) {
  const phase = metrics.phases.get(phaseName);
  if (!phase) return;

  // Add final memory snapshot
  const endMemory = takeMemorySnapshot();
  phase.memorySnapshots.push(endMemory);
  phase.memoryPeak = updateMemoryPeak(endMemory, phase.memoryPeak);
}

function finalizeBuildMetrics(instanceId: string) {
  const metrics = buildMetrics.get(instanceId);
  if (!metrics) return;

  // Finalize current phase
  if (metrics.currentPhase) {
    finalizePhase(metrics, metrics.currentPhase);
  }

  stopMemoryTracking(metrics);

  // Convert to serializable format and store globally
  const serializedPhases: PhaseMetrics[] = Array.from(
    metrics.phases.entries(),
  ).map(([name, phase]) => {
    const endTime = performance.now();
    const endMemory =
      phase.memorySnapshots[phase.memorySnapshots.length - 1] ||
      phase.startMemory;

    return {
      name,
      duration: endTime - phase.startTime,
      startTime: phase.startTime,
      endTime,
      startMemory: phase.startMemory,
      endMemory,
      memoryPeak: phase.memoryPeak,
      memorySnapshots: phase.memorySnapshots.length,
      memoryStats: calculateMemoryStats(phase.memorySnapshots),
      gcCount: 0, // TODO: Track GC events if needed
      gcTime: 0,
    };
  });

  // Store in global for retrieval
  global.__phaseTrackingMetrics = global.__phaseTrackingMetrics || {};
  global.__phaseTrackingMetrics[instanceId] = {
    phases: serializedPhases,
    overallDuration: performance.now() - metrics.overallStartTime,
    globalMemoryPeak: metrics.globalMemoryPeak,
    memorySnapshots: metrics.memorySnapshots.length,
    memoryStats: calculateMemoryStats(metrics.memorySnapshots),
  };
}

function calculateMemoryStats(snapshots: MemorySnapshot[]) {
  if (snapshots.length === 0) return null;

  const rssValues = snapshots.map((s) => s.rss);
  const heapUsedValues = snapshots.map((s) => s.heapUsed);
  const heapTotalValues = snapshots.map((s) => s.heapTotal);
  const externalValues = snapshots.map((s) => s.external);

  return {
    rss: calculateDetailedStats(rssValues),
    heapUsed: calculateDetailedStats(heapUsedValues),
    heapTotal: calculateDetailedStats(heapTotalValues),
    external: calculateDetailedStats(externalValues),
    sampleCount: snapshots.length,
  };
}

function calculateDetailedStats(values: number[]) {
  const sorted = [...values].sort((a, b) => a - b);
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  const variance =
    values.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) /
    values.length;

  return {
    min: Math.min(...values),
    max: Math.max(...values),
    mean,
    median: sorted[Math.floor(sorted.length / 2)],
    p95: sorted[Math.floor(sorted.length * 0.95)],
    p99: sorted[Math.floor(sorted.length * 0.99)],
    standardDeviation: Math.sqrt(variance),
    range: Math.max(...values) - Math.min(...values),
  };
}

export default new Reporter({
  // eslint-disable-next-line require-await
  async report({event, options}): Promise<void> {
    // Use benchmark instance ID from environment, fallback to the internal instanceId
    const instanceId =
      process.env.BENCHMARK_INSTANCE_ID || options.instanceId || 'default';

    switch (event.type) {
      case 'buildStart':
        initializeBuildMetrics(instanceId);
        break;

      case 'buildProgress':
        // eslint-disable-next-line no-case-declarations
        const metrics = buildMetrics.get(instanceId);
        if (metrics && event.phase) {
          startPhase(metrics, event.phase);
        }
        break;
      case 'buildSuccess':
      case 'buildFailure':
        finalizeBuildMetrics(instanceId);
        break;
      default:
        break;
    }
  },
});

// Export utility functions for external access
export function getPhaseMetrics(instanceId: string = 'default') {
  return global.__phaseTrackingMetrics?.[instanceId] || null;
}

export function clearPhaseMetrics(instanceId?: string) {
  if (!global.__phaseTrackingMetrics) return;

  if (instanceId) {
    delete global.__phaseTrackingMetrics[instanceId];
  } else {
    global.__phaseTrackingMetrics = {};
  }
}
