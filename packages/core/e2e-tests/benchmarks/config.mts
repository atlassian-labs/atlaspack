import type {BenchmarkOptions} from './types.mts';

// Thresholds for regression/improvement detection
export const PERFORMANCE_THRESHOLD = 5; // 5%
export const MEMORY_THRESHOLD = 15; // 15%

export const BENCHMARK_CONFIGS: BenchmarkOptions[] = [
  {
    name: 'Three.js Real Repository (JS)',
    target: 'three-js-project/index.html',
    samples: 5,
    warmupRuns: 1,
    buildOptions: {
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
        shouldOptimize: true,
      },
      shouldDisableCache: true,
    },
  },
  {
    name: 'Three.js Real Repository (Native)',
    target: 'three-js-project/index.html',
    samples: 5,
    warmupRuns: 1,
    buildOptions: {
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
        shouldOptimize: true,
      },
      shouldDisableCache: true,
      featureFlags: {
        atlaspackV3: true,
      },
    },
  },
];

export const DEFAULT_OUTPUT_DIR = './benchmark-results';
export const BASELINE_FILENAME = 'baseline-report.json';
export const CURRENT_FILENAME = 'current-report.json';

// Three.js benchmark configuration
export const THREE_JS_CONFIG = {
  branch: process.env.THREE_JS_BRANCH || 'r108',
  repoUrl:
    process.env.THREE_JS_REPO_URL || 'https://github.com/mrdoob/three.js.git',
  copies: parseInt(process.env.THREE_JS_COPIES || '10'), // Number of three.js copies to bundle
};
