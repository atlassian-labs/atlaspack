# @atlaspack/dist-differ

A tool for comparing minified JavaScript files and directories by de-minifying and showing meaningful diffs.

## Features

- Compares minified files by splitting on semicolons and commas
- Supports directory comparison with intelligent file matching by prefix
- Filters out noise from asset IDs and unminified refs
- Provides summary mode for quick overviews
- Handles ambiguous file matches with size-based disambiguation

## Installation

### From the Atlaspack repository

If you're working within the Atlaspack monorepo:

```bash
# Build the package (if not already built)
yarn workspace @atlaspack/dist-differ build:lib

# Or build everything
yarn build
```

## Running

### Using the full command

From the Atlaspack repository root:

```bash
# Development mode (uses TypeScript directly with babel-register)
yarn workspace @atlaspack/dist-differ dev:prepare
yarn workspace @atlaspack/dist-differ exec atlaspack-dist-differ file1.js file2.js

# Production mode (requires build)
yarn workspace @atlaspack/dist-differ exec atlaspack-dist-differ file1.js file2.js
```

### Setting up an alias/function (if not using global install)

If you haven't installed globally, you can set up a shell function. Add one of these to your shell configuration (`~/.zshrc`, `~/.bashrc`, etc.):

> **Note**: Use shell functions (not aliases) to properly handle command-line arguments. Aliases can cause parse errors with arguments.

**Option 1: Using yarn workspace (recommended for development)**

Replace `/path/to/atlaspack` with your actual Atlaspack repository path:

```bash
dist-differ() {
  (cd /path/to/atlaspack && yarn workspace @atlaspack/dist-differ exec atlaspack-dist-differ "$@")
}
```

Or if you're always working from within the repo:

```bash
dist-differ() {
  yarn workspace @atlaspack/dist-differ exec atlaspack-dist-differ "$@"
}
```

**Option 2: Direct path to built binary**

Replace `/path/to/atlaspack` with your actual Atlaspack repository path:

```bash
dist-differ() {
  node /path/to/atlaspack/packages/dev/dist-differ/bin/dist-differ.js "$@"
}
```

**Option 3: Auto-detect Atlaspack repo (most flexible)**

This automatically finds the Atlaspack repo from common locations:

```bash
dist-differ() {
  local atlaspack_root
  # Try to find Atlaspack repo in common locations
  if [ -d "$HOME/Work/atlassian/atlaspack" ]; then
    atlaspack_root="$HOME/Work/atlassian/atlaspack"
  elif [ -d "$HOME/atlaspack" ]; then
    atlaspack_root="$HOME/atlaspack"
  else
    echo "Error: Could not find Atlaspack repository" >&2
    return 1
  fi

  (cd "$atlaspack_root" && yarn workspace @atlaspack/dist-differ exec atlaspack-dist-differ "$@")
}
```

After adding the function, reload your shell:

```bash
source ~/.zshrc  # or ~/.bashrc
```

Now you can use `dist-differ` from anywhere with any arguments:

```bash
dist-differ file1.js file2.js
dist-differ --summary dir1/ dir2/
dist-differ --ignore-asset-ids --ignore-unminified-refs dir1/ dir2/
```

## Usage

### Compare two files

```bash
dist-differ file1.js file2.js
```

### Compare two directories

```bash
dist-differ dir1/ dir2/
```

### Options

- `--ignore-all`: Skip all ignorable differences (equivalent to all `--ignore-*` flags)
- `--ignore-asset-ids`: Skip hunks where the only differences are asset IDs
- `--ignore-unminified-refs`: Skip hunks where the only differences are unminified refs (e.g., `$e3f4b1abd74dab96$exports`, `$00042ef5514babaf$var$...`)
- `--ignore-source-map-url`: Skip hunks where the only differences are source map URLs (e.g., `//# sourceMappingURL=zh_TW.e18ec001.js.map`)
- `--ignore-swapped-variables`: Skip hunks where the only differences are swapped variable names (e.g., `t` vs `a` where functionality is identical)
- `--summary`: Show only hunk counts for changed files (directory mode only)
- `--verbose`: Show all file matches, not just mismatches (directory mode only)
- `--json`: Output results in JSON format for AI analysis
- `--disambiguation-size-threshold <val>`: Threshold for matching files by "close enough" sizes (default: 0.01 = 1%, range: 0-1)

### Examples

```bash
# Compare files with asset ID differences ignored
dist-differ --ignore-asset-ids file1.js file2.js

# Compare directories in summary mode
dist-differ --summary dir1/ dir2/

# Compare with custom size threshold for disambiguation
dist-differ --disambiguation-size-threshold 0.05 dir1/ dir2/

# Compare with both asset IDs and unminified refs ignored
dist-differ --ignore-asset-ids --ignore-unminified-refs dir1/ dir2/

# Compare ignoring all ignorable differences
dist-differ --ignore-all dir1/ dir2/

# Compare ignoring only source map URLs
dist-differ --ignore-source-map-url file1.js file2.js

# Compare ignoring swapped variables
dist-differ --ignore-swapped-variables file1.js file2.js

# Verbose mode to see all file matches
dist-differ --verbose dir1/ dir2/

# Output JSON format for programmatic analysis
dist-differ --json file1.js file2.js
dist-differ --json --ignore-all dir1/ dir2/
```

## Development

### Building

The package is built as part of the main Atlaspack build process:

```bash
# Build just this package
yarn workspace @atlaspack/dist-differ build:lib

# Or build everything
yarn build
```

### Development Mode

For development, you can use the dev script which uses TypeScript directly:

```bash
yarn workspace @atlaspack/dist-differ dev:prepare
yarn workspace @atlaspack/dist-differ exec atlaspack-dist-differ [args...]
```

This uses `babel-register` to transpile TypeScript on the fly, so you don't need to rebuild after making changes.

## JSON Output for AI Analysis

The `--json` flag produces structured JSON output designed for programmatic analysis, particularly by AI agents. The JSON format includes:

- **Categorized hunks**: Each change is classified as "meaningful" or "harmless" with confidence scores
- **Context**: Surrounding code lines for each change
- **Normalized representations**: Shows what code looks like after normalization (useful for verifying harmless changes)
- **Semantic analysis**: For meaningful changes, includes change type and impact assessment

### Sample AI Analysis Prompt

When using the JSON output with an AI agent like Cursor, you can use this prompt template:

```
I have a JSON report from dist-differ comparing two minified JavaScript builds.
Please analyze the report and determine:

1. Are there any meaningful changes (not just harmless reordering, variable swaps, or asset ID changes)?
2. For each meaningful change, what is the nature of the change (function modification, dependency change, logic change, etc.)?
3. What is the potential impact of these changes (low/medium/high)?
4. Should I be concerned about these differences, or are they all harmless build artifacts?

Here is the JSON report:

[Paste the JSON output from dist-differ --json]

Please provide:
- A summary of meaningful vs harmless changes
- Detailed analysis of each meaningful change
- Recommendations on whether action is needed
```

### Example JSON Output Structure

```json
{
  "metadata": {
    "file1": "/path/to/file1.js",
    "file2": "/path/to/file2.js",
    "comparisonDate": "2024-01-15T10:30:00Z",
    "options": {
      "ignoreAssetIds": false,
      "ignoreUnminifiedRefs": false,
      "ignoreSourceMapUrl": false,
      "ignoreSwappedVariables": false
    }
  },
  "summary": {
    "totalHunks": 5,
    "meaningfulHunks": 2,
    "harmlessHunks": 3,
    "identical": false
  },
  "files": [
    {
      "path": "relative/path/to/file.js",
      "status": "different",
      "hunks": [
        {
          "id": "hunk-1",
          "category": "harmless",
          "harmlessType": "swapped_variables",
          "confidence": 0.90,
          "changes": [...],
          "normalized": {...}
        },
        {
          "id": "hunk-2",
          "category": "meaningful",
          "confidence": 1.0,
          "changes": [...],
          "analysis": {
            "semanticChange": true,
            "changeType": "function_definition",
            "impact": "high"
          }
        }
      ]
    }
  ]
}
```

## How it works

1. **De-minification**: Files are split on semicolons and commas to make diffs more readable
2. **Normalization**: Asset IDs and unminified refs can be normalized to filter out noise
3. **File Matching**: When comparing directories, files are matched by prefix (name before hash)
4. **Size Disambiguation**: When multiple files match a prefix, size-based matching helps resolve ambiguity
5. **JSON Output**: When `--json` is used, changes are categorized and structured for programmatic analysis
