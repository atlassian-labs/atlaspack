# Compiled CSS Transformer Comparison Tool

This tool compares the output of two Compiled CSS transformers:
- **Baseline**: `@compiled/parcel-transformer` - The reference implementation
- **Experiment**: `@atlaspack/transformer-compiled-css-in-js` - The Atlaspack implementation

## Overview

The tool uses Atlaspack's `unstable_transform` API to process fixture files from the SWC Compiled CSS plugin test suite and compare the outputs of both transformers. This helps ensure that the Atlaspack Compiled CSS transformer produces equivalent output to the reference Parcel transformer.

## Usage

### Basic Usage

```bash
# Run comparison on all fixtures
yarn compare-compiled-css

# Or directly with node
node scripts/compare-compiled-css.js
```

### With Custom Options

```bash
# Compare specific fixture directory
node scripts/compare-compiled-css.js \
  --fixtures-path crates/atlassian-swc-compiled-css/tests/fixtures/basic-css \
  --output-dir ./custom-results

# Use custom config
node scripts/compare-compiled-css.js \
  --config-path ./custom-compiledcssrc.json \
  --output-dir ./results-with-custom-config
```

### Command Line Options

- `--fixtures-path <path>` - Path to fixtures directory (default: `crates/atlassian-swc-compiled-css/tests/fixtures`)
- `--config-path <path>` - Path to .compiledcssrc config file (default: `./.compiledcssrc`)
- `--output-dir <path>` - Directory to write comparison results (default: `./comparison-results`)
- `--fixture-glob <pattern>` - Glob pattern for fixture files (default: `**/in.jsx`)
- `--help` - Show help message

## Configuration

The tool looks for a `.compiledcssrc` configuration file to configure the Compiled CSS transformer. The default configuration includes:

```json
{
  "importSources": ["@compiled/react"],
  "extractToFile": false,
  "transformCssMap": true,
  "optimizeCss": true,
  "cache": true,
  "sourceMaps": true
}
```

## Output

The tool generates several output files:

### Per-fixture Results

For each fixture, the tool creates a directory with:

- `baseline-compiled-parcel.js` - Output from @compiled/parcel-transformer
- `experiment-atlaspack.js` - Output from @atlaspack/transformer-compiled-css-in-js
- `baseline-compiled-parcel.style-rules.json` - Style rules from baseline (if any)
- `experiment-atlaspack.style-rules.json` - Style rules from experiment (if any)
- `comparison.json` - Metadata about the comparison result
- `diff.txt` - Side-by-side diff when outputs don't match

### Summary Reports

- `summary.json` - JSON summary with overall statistics and per-fixture results
- `report.md` - Markdown report with detailed breakdown of issues

### Example Output Structure

```
comparison-results/
├── summary.json
├── report.md
├── basic-css/
│   ├── baseline-compiled-parcel.js
│   ├── experiment-atlaspack.js
│   ├── comparison.json
│   └── diff.txt
├── styled-component/
│   ├── baseline-compiled-parcel.js
│   ├── experiment-atlaspack.js
│   └── comparison.json
└── ...
```

## Fixture Structure

The tool expects fixtures to be organized as follows:

```
fixtures/
├── basic-css/
│   ├── in.jsx          # Input file to transform
│   ├── out.js          # Expected output (preferred)
│   ├── actual.js       # Alternative expected output
│   └── style-rules.json # Optional style rules
├── another-fixture/
│   ├── in.tsx
│   └── expected.js
└── ...
```

The tool will look for expected output in this order:
1. `out.js`
2. `actual.js` 
3. `expected.js`

## Exit Codes

- `0` - All fixtures processed successfully and match expected output
- `1` - One or more fixtures failed to process or don't match expected output

## Integration with CI

The tool can be used in CI pipelines to ensure transformer compatibility:

```bash
# Run comparison and fail if there are mismatches
yarn compare-compiled-css
if [ $? -eq 1 ]; then
  echo "❌ Compiled CSS transformer output doesn't match expected results"
  exit 1
fi
```

## Troubleshooting

### Common Issues

1. **Both transformers produce identical output**: This likely means neither transformer is being applied correctly. Check:
   - Feature flags are enabled (e.g., `compiledCssInJsTransformer`)
   - Transformer packages are installed and available
   - Atlaspack configuration is correctly set up
   - The temporary config files are being generated correctly

2. **Transformer not being applied**: Ensure the transformer configuration is correct in the generated Atlaspack config files.

3. **No fixtures found**: Check that the fixtures path is correct and contains `in.jsx`/`in.js`/`in.ts`/`in.tsx` files.

4. **Package resolution errors**: Ensure both `@compiled/parcel-transformer` and `@atlaspack/transformer-compiled-css-in-js` are installed.

### Debug Mode

To see detailed transformation information, you can modify the script to log intermediate steps or use Atlaspack's debug options.

## Future Enhancements

Potential improvements to consider:

1. **Parallel processing** - Process multiple fixtures concurrently
2. **Incremental comparisons** - Only process changed fixtures
3. **Integration with @compiled/parcel-transformer** - Add direct comparison with Parcel's transformer
4. **Performance benchmarking** - Time transformation performance
5. **Visual diff reports** - HTML reports with syntax highlighting
6. **Watch mode** - Continuously monitor for changes and re-run comparisons