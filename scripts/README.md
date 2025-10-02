# Scripts

This directory contains utility scripts for Atlaspack development.

## dist-differ

Enhanced directory comparison tool with intelligent file matching and JavaScript formatting for readable diffs.

### Features

- **Content Hash Stripping**: Automatically matches files with different content hashes (e.g., `bundle.abc123.js` ↔ `bundle.def456.js`)
- **Smart File Matching**: Three-tier matching strategy (exact path → normalized path → size-based)
- **JavaScript Formatting**: Automatically formats minified JavaScript for readable diffs
- **Configurable Size Threshold**: Adjustable tolerance for file size differences
- **Comprehensive Reporting**: Clear summary of matches, differences, and missing files

### Usage

#### TypeScript with Node.js experimental support (Recommended)
```bash
# Run directly from anywhere - no compilation needed!
node --experimental-strip-types scripts/dist-differ.ts <dir1> <dir2> [sizeThresholdPercent]

# From other projects
node --experimental-strip-types /path/to/atlaspack/scripts/dist-differ.ts <dir1> <dir2> [sizeThresholdPercent]
```

#### Alternative: Standalone JavaScript
```bash
# Fallback for older Node.js versions
node scripts/dist-differ.js <dir1> <dir2> [sizeThresholdPercent]
```

#### Alternative: npm scripts (for development)
```bash
cd scripts
npm run dist-differ <dir1> <dir2> [sizeThresholdPercent]
```

### Examples

```bash
# Compare two build directories with default 5% size threshold
node --experimental-strip-types scripts/dist-differ.ts packages/entry-point/dist packages/entry-point/dist-control

# Compare with 10% size threshold for more lenient matching
node --experimental-strip-types scripts/dist-differ.ts dist1 dist2 10

# From other projects
node --experimental-strip-types /path/to/atlaspack/scripts/dist-differ.ts my-dist my-dist-control

# Using JavaScript fallback (older Node.js)
node scripts/dist-differ.js dist1 dist2

# Example output showing JavaScript formatting in action:
# ❌ bundle.abc123.js -> bundle.def456.js (normalized match): Content differs
# Diff:
# --- bundle.abc123.js
# +++ bundle.def456.js
# @@ -1,7 +1,7 @@
#  function test(){
#  var a=1;
# -var b=2;
# -return a+b;
# +var c=3;
# +return a+c;
```

### Parameters

- `<dir1>`: First directory to compare
- `<dir2>`: Second directory to compare  
- `[sizeThresholdPercent]`: Optional percentage threshold for size-based matching (default: 5)

### Exit Codes

- `0`: All matched files are identical
- `1`: Found differences, missing files, or errors

### Testing

Run the comprehensive test suite:

```bash
cd scripts
npm run test:dist-differ
```

The test suite includes 40+ tests covering:
- Content hash stripping patterns
- File matching algorithms
- JavaScript formatting logic
- Size threshold calculations
- Integration scenarios
- Error handling

### How It Works

1. **File Discovery**: Recursively scans both directories, excluding `.js.map` files
2. **Content Hash Normalization**: Strips hashes like `.abc123.` from filenames
3. **Smart Matching**: 
   - First tries exact path matches
   - Then tries normalized path matches (ignoring hashes)
   - Finally picks the closest size match if multiple candidates exist
4. **Content Comparison**: 
   - For JavaScript files: Detects minification and formats for readable diffs
   - For other files: Standard diff comparison
5. **Reporting**: Detailed summary with match types and file differences

### Advanced Features

#### JavaScript Formatting
The script automatically detects and formats minified JavaScript files to make diffs readable:

- **Detection**: Based on file extension and content patterns
- **Formatting**: Adds strategic newlines after `;`, `{`, `}`, and `,`
- **Performance**: Fast regex-based approach, no AST parsing
- **Safety**: Only formats files that appear truly minified

#### Content Hash Patterns
Recognizes and strips various hash patterns:
- `bundle.123456.js` → `bundle.js`
- `styles.abcdef.css` → `styles.css` 
- `script.hash123.min.js` → `script.min.js`
- Requires 6+ character hashes to avoid false positives

#### Size-Based Matching
When multiple files have the same normalized name:
- Calculates percentage difference based on larger file size
- Selects the file with the closest size match
- Configurable threshold prevents incorrect matches