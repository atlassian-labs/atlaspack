# Atlaspack Benchmarks

This directory contains the performance benchmarking system for Atlaspack. It's designed to track both build performance and memory usage to detect regressions and improvements.

## Features

- **Multiple Samples**: Run benchmarks multiple times to get consistent, statistically meaningful results
- **Memory Tracking**: Monitor memory usage throughout the build process
- **Phase-based Metrics**: Track performance of different build phases (currently build phase, can be extended)
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

# Run the a specific benchmark
yarn benchmark --test="three.js"

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

1. **Simple Project Build** - Basic project with minimal dependencies
2. **Project with Async Imports** - Tests dynamic import handling
3. **Project with Shared Bundles** - Tests bundle splitting optimization
4. **Project with Conditional Bundles (Production)** - Tests conditional bundling in production mode
5. **Compiled CSS-in-JS (Extracted)** - Tests CSS-in-JS compilation with extraction
6. **Three.js Real Repository** - Uses the actual three.js library repository (cloned from GitHub) with multiple copies to test bundling performance on a real-world large JavaScript library. This benchmark tests:
   - Real-world large codebase bundling performance
   - Complex dependency resolution with actual three.js modules
   - Production mode optimizations on a substantial JavaScript library
   - Memory usage under heavy bundling loads
   - Tree-shaking effectiveness on a large, modular library

## CI Integration

The benchmark system integrates with GitHub Actions:

- **Automatic Trigger**: Runs on PRs that modify core packages
- **Baseline Comparison**: Automatically compares against main branch
- **PR Comments**: Posts results directly to the PR
- **Regression Detection**: Fails the build if significant regressions are detected

### Regression Thresholds

- **Performance**: 5% increase in build time
- **Memory**: 10% increase in peak memory usage

## Output Format

### JSON Report Structure

```json
{
  "results": [
    {
      "name": "Simple Project Build",
      "target": "simple-project/index.html",
      "samples": [...],
      "stats": {
        "mean": 1234.56,
        "median": 1230.12,
        "min": 1200.45,
        "max": 1260.78,
        "standardDeviation": 15.23,
        "memoryPeakMean": 52428800,
        "memoryPeakMedian": 52000000
      },
      "timestamp": "2024-01-01T12:00:00.000Z",
      "gitSha": "abc123def456",
      "gitBranch": "feature-branch"
    }
  ],
  "environment": {
    "nodeVersion": "v20.0.0",
    "platform": "linux",
    "arch": "x64",
    "cpu": "Intel(R) Core(TM) i7-9750H CPU @ 2.60GHz",
    "memory": 17179869184
  },
  "timestamp": "2024-01-01T12:00:00.000Z"
}
```

### Markdown Report

The system generates markdown reports suitable for GitHub comments:

```markdown
# 📊 Benchmark Results

| Test                       | Duration | Memory Peak | vs Baseline                    | Status         |
| -------------------------- | -------- | ----------- | ------------------------------ | -------------- |
| Simple Project Build       | 1.23s    | 50.00MB     | +2.15% duration, +1.23% memory | 🟡 Neutral     |
| Project with Async Imports | 1.45s    | 55.20MB     | -5.67% duration, -2.34% memory | 🟢 Improvement |
```

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
      // Custom build options
    },
  },
];
```

### Adding Phase Tracking

The current system tracks the build as a single phase. To add more granular phase tracking, modify the `runBenchmark` function in `runner.mts` to instrument different parts of the build process.

## Development

### Local Testing

```bash
# Test with a quick run (fewer samples)
yarn benchmark --samples=2 --test="simple"

# Test baseline comparison
yarn benchmark --samples=2
mv benchmark-results/current-report.json benchmark-results/baseline-report.json
# Make some changes...
yarn benchmark --samples=2 --baseline=benchmark-results/baseline-report.json
```

### CI Testing

The benchmark workflow can be triggered manually with:

```bash
gh workflow run benchmark.yml
```

## Performance Tips

- The system automatically runs warmup iterations to reduce JIT compilation effects
- Memory measurements include periodic sampling during the build
- Garbage collection is triggered between samples when available (`--expose-gc`)
- Results are more stable on dedicated CI runners vs shared infrastructure
