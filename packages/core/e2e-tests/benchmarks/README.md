# Atlaspack Benchmarks

This directory contains the performance benchmarking system for Atlaspack. It provides comprehensive tracking of build performance, memory usage (both JavaScript and native), and detailed phase-level metrics to detect regressions and improvements.

## Features

- **Multiple Samples**: Run benchmarks multiple times to get consistent, statistically meaningful results
- **Comprehensive Memory Tracking**: Monitor both JavaScript heap and native memory allocation throughout the build process
- **Phase-based Metrics**: Track detailed performance and memory usage of different build phases (resolving, transforming, bundling, packaging, optimizing)
- **Native Memory Profiling**: Track native Rust memory allocations with counter reset between samples
- **High-Resolution Sampling**: 25ms memory sampling intervals during builds for detailed memory profiles
- **JSON Reports**: Save structured benchmark data for analysis and comparison
- **Baseline Comparison**: Compare current performance against a baseline (typically main branch)
- **GitHub Integration**: Automatic PR comments with benchmark results
- **Regression Detection**: Automatically detect performance and memory regressions

## Usage

### Running Benchmarks Locally

```bash
# Run all benchmarks with default settings (5 samples each)
yarn benchmark

# Run with custom number of samples
yarn benchmark --samples=10

# Run a specific benchmark (partial name matching)
yarn benchmark --test="three"

# Compare against a specific baseline
yarn benchmark --baseline=./path/to/baseline-report.json

# Generate GitHub comment format
yarn benchmark --github-comment
```

### Available Command Line Options

- `--output=<path>`: Output directory for reports (default: `./benchmark-results`)
- `--baseline=<path>`: Path to baseline report for comparison
- `--github-comment`: Generate GitHub comment format
- `--test=<name>`: Run only benchmarks matching the given name
- `--samples=<number>`: Number of samples per benchmark (default: 5)

### Available Benchmarks

The system currently includes these benchmark configurations:

1. **Three.js Real Repository** - Uses the actual three.js library repository (cloned from GitHub) with configurable copies to test bundling performance on a real-world large JavaScript library. This benchmark tests:
   - Real-world large codebase bundling performance
   - Complex dependency resolution with actual three.js modules
   - Production mode optimizations with scope hoisting enabled
   - Memory usage under heavy bundling loads (both JS heap and native allocations)
   - Tree-shaking effectiveness on a large, modular library
   - All build phases: resolving, transforming, bundling, packaging, and optimizing

## CI Integration

The benchmark system integrates with GitHub Actions:

- **Automatic Trigger**: Runs on PRs that modify core packages
- **Baseline Comparison**: Automatically compares against main branch
- **PR Comments**: Posts results directly to the PR
- **Regression Detection**: Fails the build if significant regressions are detected

### Regression Thresholds

- **Performance**: 5% increase in build time
- **Memory**: 10% increase in peak memory usage

## Extending the System

### Adding New Benchmarks

Add new benchmark configurations to `config.mts`:

```typescript
export const BENCHMARK_CONFIGS: BenchmarkOptions[] = [
  // ... existing configs
  {
    name: 'My New Benchmark',
    target: 'my-test-project/index.html',
    samples: 5,
    warmupRuns: 2,
    buildOptions: {
      mode: 'production',
      defaultTargetOptions: {
        shouldScopeHoist: true,
        shouldOptimize: true,
      },
      shouldDisableCache: true,
    },
  },
];
```

## Development

### Local Testing

```bash
# Test with a quick run (fewer samples)
yarn workspace @atlaspack/e2e-tests benchmark --samples=1 --test="three"

# Test baseline comparison
yarn workspace @atlaspack/e2e-tests benchmark --samples=2
mv packages/core/e2e-tests/benchmark-results/current-report.json packages/core/e2e-tests/benchmark-results/baseline-report.json
# Make some changes...
yarn workspace @atlaspack/e2e-tests benchmark --samples=2 --baseline=packages/core/e2e-tests/benchmark-results/baseline-report.json
```

### CI Testing

The benchmark workflow can be triggered manually with:

```bash
gh workflow run benchmark.yml
```
