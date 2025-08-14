# Reused Inline Requires Runtime Benchmark

This benchmark tests the **runtime performance impact** of the `reusedInlineRequires` feature flag in Atlaspack. It compares bundle sizes and browser runtime performance with the feature enabled and disabled, focusing on how the optimization affects actual user experience.

## What is reusedInlineRequires?

The `reusedInlineRequires` feature is an optimization in Atlaspack's inline requires optimizer that reuses require statements across scopes instead of inlining them multiple times. This can potentially:

- **Reduce bundle size** by eliminating duplicate require statements
- **Improve runtime performance** by reducing repetitive module loading code
- **Speed up builds** by optimizing the bundling process

## Benchmark Structure

The benchmark includes:

1. **Multiple utility modules** (`stringUtils`, `arrayUtils`, `objectUtils`, `mathUtils`, `dateUtils`) that are heavily used throughout the application
2. **Feature modules** (A, B, C, D, E) that extensively use these utilities in different patterns
3. **React components** that exercise the features both synchronously and asynchronously
4. **Lazy-loaded modules** to test dynamic import optimization

This structure ensures that the inline requires optimizer has many opportunities to either inline or reuse require statements.

## Usage

### Quick Start

```bash
# Install dependencies
yarn install

# Install Playwright browsers
npx playwright install chromium

# Run the full runtime benchmark
yarn benchmark

# Or run individual commands
yarn build:both    # Build both versions
yarn serve         # Start development server
```

### Environment Variables

- `BENCHMARK_RUNS`: Number of runs per configuration (default: 5)
- `BROWSER_TIMEOUT`: Browser test timeout in ms (default: 30000)

### Manual Testing

You can also manually compare the builds:

```bash
# Build with feature OFF
yarn build:off

# Build with feature ON  
yarn build:on

# Start comparison server
yarn serve
```

Then visit:
- http://localhost:3000 - Side-by-side comparison
- http://localhost:3000/dist-off/index.html - Feature OFF
- http://localhost:3000/dist-on/index.html - Feature ON

## Benchmark Results

The benchmark measures:

### Bundle Metrics  
- **Total bundle size**: Combined size of all JavaScript files
- **Number of files**: Count of generated JS bundles
- **Individual file sizes**: Size breakdown per bundle

### Browser Metrics
- **Load time**: Time from navigation start to load complete
- **Parse time**: Time spent parsing JavaScript
- **Execute time**: Time spent executing JavaScript
- **First paint/contentful paint**: Visual performance markers
- **Memory usage**: JavaScript heap size
- **Feature execution time**: Performance of running application features

## Interpreting Results

### Expected Outcomes with reusedInlineRequires=true

**Bundle Size**: Should be smaller due to reduced duplicate require statements  
**Runtime Performance**: Should be faster due to less repetitive module loading code
**Memory Usage**: May be lower due to reduced code duplication
**Load Times**: Should improve due to smaller bundles and optimized execution

### Sample Output

```
BUNDLE METRICS  
┌─────────────────────┬─────────────────┬─────────────────┬─────────────────┐
│ Bundle Metric       │ Feature OFF     │ Feature ON      │ Change          │
├─────────────────────┼─────────────────┼─────────────────┼─────────────────┤
│ Total Bundle Size   │ 245.7 KB        │ 231.2 KB        │ -5.9%           │
│ Number of JS Files  │ 5               │ 5               │ 0               │
└─────────────────────┴─────────────────┴─────────────────┴─────────────────┘

RUNTIME PERFORMANCE
┌─────────────────────┬─────────────────┬─────────────────┬─────────────────┐
│ Runtime Metric      │ Feature OFF     │ Feature ON      │ Improvement     │
├─────────────────────┼─────────────────┼─────────────────┼─────────────────┤
│ Load Time           │ 342.3ms         │ 318.7ms         │ +6.9%           │
│ Parse Time          │ 89.2ms          │ 82.1ms          │ +8.0%           │
│ Execute Time        │ 156.8ms         │ 142.3ms         │ +9.2%           │
│ First Contentful Paint │ 285.1ms      │ 263.4ms         │ +7.6%           │
│ JS Heap Size        │ 12.4 MB         │ 11.8 MB         │ +4.8%           │
└─────────────────────┴─────────────────┴─────────────────┴─────────────────┘
```

## Implementation Details

### Test Application

The benchmark application is a React app that:

1. **Imports multiple utility modules** at the top level
2. **Uses utilities extensively** in feature classes with the `@timed` decorator
3. **Exercises both sync and async** code paths
4. **Includes lazy-loaded components** to test dynamic imports
5. **Provides interactive features** that can be triggered manually

### Measurement Strategy

**Bundle Analysis**: Analyzes the generated JavaScript files for size and count

**Browser Performance**: Uses Playwright to load the application and measure:
- Navigation timing API metrics
- Paint timing measurements  
- Memory usage via performance.memory
- Custom performance markers

### Utility Modules

Each utility module (`stringUtils`, `arrayUtils`, etc.) wraps Lodash functions to ensure consistent require patterns. This creates many opportunities for the inline requires optimizer to either:

- **Inline the requires** (feature OFF) - Each usage site gets its own require statement
- **Reuse requires** (feature ON) - Common requires are hoisted and reused

## Files Structure

```
benchmarks/reused-inline-requires/
├── README.md                 # This documentation
├── package.json             # Dependencies and scripts
├── .parcelrc               # Atlaspack configuration
├── src/
│   ├── index.html          # Entry HTML file
│   ├── index.tsx           # Main React application
│   ├── utils/              # Utility modules (heavily used)
│   │   ├── performance.ts  # Performance tracking
│   │   ├── stringUtils.ts  # String utilities
│   │   ├── arrayUtils.ts   # Array utilities
│   │   ├── objectUtils.ts  # Object utilities
│   │   ├── mathUtils.ts    # Math utilities
│   │   └── dateUtils.ts    # Date utilities
│   └── features/           # Feature modules
│       ├── featureA.tsx    # Feature A (sync processing)
│       ├── featureB.tsx    # Feature B (analysis)
│       ├── featureC.tsx    # Feature C (inventory)
│       ├── featureD.tsx    # Feature D (big data, lazy)
│       └── featureE.tsx    # Feature E (time series, lazy)
├── scripts/
│   ├── benchmark.mjs       # Main benchmark runner
│   └── serve.mjs          # Development server
├── dist-off/              # Build output (feature OFF)
├── dist-on/               # Build output (feature ON)
└── benchmark-results.json # Detailed results (generated)
```

## Contributing

To modify the benchmark:

1. **Add more utilities**: Create new utility modules in `src/utils/`
2. **Add more features**: Create new feature modules in `src/features/`
3. **Modify measurement**: Update the benchmark script in `scripts/benchmark.mjs`
4. **Change test patterns**: Modify the React components to test different require patterns

The benchmark is designed to be comprehensive but can be extended to test specific optimization scenarios.
