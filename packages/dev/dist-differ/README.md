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
- `--json`: Output results in JSON format for AI analysis (uses streaming for large outputs)
- `--mcp`: Start an MCP server for AI agent queries (requires comparison paths)
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

# Start MCP server for AI agent queries
dist-differ --mcp dir1/ dir2/
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

## MCP Server for AI Agents

The `--mcp` flag starts a Model Context Protocol (MCP) server that allows AI agents to query dist diff data interactively. This is useful when you want an AI agent to analyze differences without loading the entire JSON output into memory.

### Usage

```bash
# Start MCP server (optionally with an initial comparison)
dist-differ --mcp                    # Start server without initial comparison
dist-differ --mcp dir1/ dir2/        # Start server with initial comparison
```

The server will:

1. Optionally run an initial comparison if two paths are provided
2. Start an MCP server on stdio (stdin/stdout)
3. Accept queries from AI agents via the MCP protocol

**Note**: You can start the server without providing paths and use the `compare` tool to run comparisons on-demand.

### Configuring in Cursor

To use the dist-differ MCP server in Cursor, you need to add it to your MCP configuration file. The configuration file is typically located at `~/.cursor/mcp.json` (or `%APPDATA%\Cursor\mcp.json` on Windows).

**Important**: Since the MCP server requires specific directories/files to compare, you have two options:

#### Option 1: Manual Start (Recommended)

Start the MCP server manually in a terminal before using it in Cursor:

```bash
# In your terminal
dist-differ --mcp /path/to/dir1 /path/to/dir2
```

Then configure Cursor to connect to a running stdio MCP server. However, note that Cursor's MCP integration typically expects to start the server itself, so this approach may require custom setup.

#### Option 2: Per-Project Configuration (Recommended)

You can configure the MCP server in Cursor to start automatically. Since the server now supports on-demand comparisons via the `compare` tool, you can start it without specifying directories:

```json
{
  "mcpServers": {
    "dist-differ": {
      "command": "node",
      "args": [
        "/path/to/atlaspack/packages/dev/dist-differ/bin/dist-differ.js",
        "--mcp"
      ]
    }
  }
}
```

Replace `/path/to/atlaspack/packages/dev/dist-differ/bin/dist-differ.js` with the actual path to the dist-differ binary.

**Note**: With the `compare` tool, you can now run comparisons on-demand without restarting the server or updating the configuration. Simply use the `compare` tool with the paths you want to compare.

If you want to start with an initial comparison, you can still provide the directories:

```json
{
  "mcpServers": {
    "dist-differ": {
      "command": "node",
      "args": [
        "/path/to/atlaspack/packages/dev/dist-differ/bin/dist-differ.js",
        "--mcp",
        "/path/to/dir1",
        "/path/to/dir2"
      ]
    }
  }
}
```

#### Using yarn workspace (if working in Atlaspack repo)

If you're working within the Atlaspack repository, you can use:

```json
{
  "mcpServers": {
    "dist-differ": {
      "command": "yarn",
      "args": [
        "workspace",
        "@atlaspack/dist-differ",
        "exec",
        "atlaspack-dist-differ",
        "--mcp",
        "/path/to/dir1",
        "/path/to/dir2"
      ],
      "cwd": "/path/to/atlaspack"
    }
  }
}
```

### Available MCP Tools

The MCP server provides the following tools for AI agents:

- **`compare`**: Run a dist diff analysis between two files or directories. **Returns summary data immediately** for quick analysis, then stores the full results in memory for selective querying. For large comparisons (100s of MB), the summary allows you to decide whether to explore further before querying specific areas. Parameters:
  - `path1`: First file or directory path to compare
  - `path2`: Second file or directory path to compare
  - `ignoreAssetIds` (optional): Ignore asset ID differences
  - `ignoreUnminifiedRefs` (optional): Ignore unminified ref differences
  - `ignoreSourceMapUrl` (optional): Ignore source map URL differences
  - `ignoreSwappedVariables` (optional): Ignore swapped variable differences

  **Note**: The `compare` tool returns summary statistics immediately (total hunks, meaningful vs harmless changes, file counts). The full diff data is stored in memory for use by other tools, but you can use selective queries to explore specific files or changes without loading everything at once.

- **`get-summary`**: Get a summary of the current comparison including total hunks, meaningful vs harmless changes, and file counts
- **`list-files`**: List all files that were compared, showing their status and hunk counts
- **`get-file-details`**: Get detailed information about differences in a specific file. For files with many hunks (>10), shows a sample (first 3 and last 3 hunks) to give you the "vibe" of the changes rather than exhaustive details. Use `get-next-hunks` for progressive iteration.
- **`get-meaningful-changes`**: Get a **statistics-only summary** of meaningful (non-harmless) changes. Returns counts, change types, impact distribution, and top files - **never returns hunk details** to avoid context window issues. Use `get-next-meaningful-hunks` for progressive iteration through actual changes.
- **`search-changes`**: Search for specific text patterns in the changes across all files
- **`get-next-hunks`**: **Progressive iteration** - Get the next batch of hunks for a specific file. The MCP server maintains iterator state, allowing you to navigate through large diffs incrementally. Parameters:
  - `filePath`: The file path to iterate through
  - `batchSize` (optional): Number of hunks to return per call (default: 5)
  - `reset` (optional): Reset iterator to beginning (default: false)
- **`get-next-meaningful-hunks`**: **Progressive iteration** - Get the next batch of meaningful hunks across all files. Maintains global iterator state for reviewing meaningful changes incrementally. Parameters:
  - `batchSize` (optional): Number of hunks to return per call (default: 5)
  - `reset` (optional): Reset iterator to beginning (default: false)
- **`get-hunk-by-id`**: Get detailed information about a specific hunk by its ID. Useful when you want to examine a particular change in detail. Parameters:
  - `hunkId`: The hunk ID (e.g., "hunk-1")
  - `filePath` (optional): The file path to search in (searches all files if not provided)
- **`quit`**: Forces the MCP server to quit and exit. Use this when you want to stop the server. No parameters required.

### Example AI Agent Usage

An AI agent can connect to the MCP server and use these tools to:

- Quickly identify if there are any meaningful changes
- Get details about specific files or hunks
- Search for specific patterns in the code changes
- Analyze the impact of changes without loading the entire JSON report

### Sample AI Analysis Prompt (MCP Server)

When using the MCP server with an AI agent like Cursor, you can use this prompt template:

```
Please run a dist diff analysis between <path1> and <path2> using the MCP server's compare tool, then analyze the differences and determine:
```

Or if you've already run a comparison:

```
I have a dist-differ MCP server running that has compared two minified JavaScript builds.
Please use the available MCP tools to analyze the differences and determine:

1. Are there any meaningful changes (not just harmless reordering, variable swaps, or asset ID changes)?
   - Use the `get-summary` tool to get an overview
   - Use the `get-meaningful-changes` tool to see all meaningful changes

2. For each meaningful change, what is the nature of the change?
   - Use `get-file-details` to examine specific files
   - Look at the change type and impact assessment in each hunk

3. What is the potential impact of these changes (low/medium/high)?
   - Review the impact field in meaningful hunks
   - Consider the types of changes (function modifications, dependency changes, etc.)

4. Should I be concerned about these differences, or are they all harmless build artifacts?
   - Compare meaningful vs harmless hunk counts
   - Review the specific changes to assess risk

Please provide:
- A summary of meaningful vs harmless changes
- Detailed analysis of each meaningful change
- Recommendations on whether action is needed
- If you find specific patterns, use `search-changes` to investigate further
```

**Note**: The MCP server must be running (started with `dist-differ --mcp dir1/ dir2/`) for the AI agent to connect and use these tools.

### Handling Large Comparisons

The MCP server is optimized for large comparisons with **progressive iteration**:

- **State management**: The server maintains iterator state for each file and globally, allowing you to navigate through diffs incrementally
- **Progressive iteration tools**: Use `get-next-hunks` and `get-next-meaningful-hunks` to review changes in small batches (default: 5 hunks at a time)
- **Early exit for very large files**: Files with 1000+ hunks are truncated during processing to prevent the tool from getting stuck
- **Sampling for overview**: When displaying hunks in summary tools, files with >10 hunks show a sample (first 3 and last 3) to give you the "vibe" of the changes
- **Summary-first approach**: The `compare` tool returns summary statistics immediately, allowing you to decide if further exploration is needed
- **Selective querying**: Use specific tools like `get-hunk-by-id` or `search-changes` to explore targeted areas

**Recommended workflow for large comparisons**:

1. Run `compare` to get summary statistics
2. Use `get-summary` for high-level overview
3. Use `list-files` to see which files have changes
4. Use `get-next-meaningful-hunks` to review meaningful changes incrementally (5 at a time)
5. For specific files, use `get-next-hunks <filePath>` to iterate through all hunks
6. Use `get-hunk-by-id` to examine specific hunks in detail
7. Use `search-changes` to find specific patterns

This progressive approach prevents the tool from getting stuck in loops and avoids overwhelming the AI agent with too much data at once.

### Streaming JSON Output

When using `--json`, the output uses a streaming approach that:

- First attempts to use `JSON.stringify` for fast output of small reports
- Falls back to incremental JSON writing if the report is too large to stringify
- Prevents memory issues when comparing very large directories

## How it works

1. **De-minification**: Files are split on semicolons and commas to make diffs more readable
2. **Normalization**: Asset IDs and unminified refs can be normalized to filter out noise
3. **File Matching**: When comparing directories, files are matched by prefix (name before hash)
4. **Size Disambiguation**: When multiple files match a prefix, size-based matching helps resolve ambiguity
5. **JSON Output**: When `--json` is used, changes are categorized and structured for programmatic analysis
6. **MCP Server**: When `--mcp` is used, an MCP server is started for interactive AI agent queries
