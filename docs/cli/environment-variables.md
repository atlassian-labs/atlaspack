# Environment Variables

Atlaspack supports various environment variables to configure its behavior across all commands.

## Build Performance

### `ATLASPACK_WORKERS`

Number of worker threads to use for parallel processing.

- **Default**: `Math.min(4, Math.ceil(cpuCount() / 2))`
- **Usage**: `ATLASPACK_WORKERS=8 atlaspack build src/index.html`

### `ATLASPACK_MAX_CONCURRENT_CALLS`

Maximum concurrent calls per worker.

- **Default**: `30`
- **Usage**: `ATLASPACK_MAX_CONCURRENT_CALLS=50 atlaspack build src/index.html`

### `ATLASPACK_WORKER_BACKEND`

Worker backend type for parallel processing.

- **Values**: `'threads'` | `'process'`
- **Default**: Auto-detected (prefers threads if available)
- **Usage**: `ATLASPACK_WORKER_BACKEND=process atlaspack build src/index.html`

### `ATLASPACK_NATIVE_THREADS`

Number of native threads for Rust operations.

- **Default**: auto-detected based on execution environment
- **Usage**: `ATLASPACK_NATIVE_THREADS=4 atlaspack build src/index.html`

### `ATLASPACK_NAPI_WORKERS`

Number of NAPI workers for native operations.

- **Default**: Auto-detected based on available threads
- **Usage**: `ATLASPACK_NAPI_WORKERS=8 atlaspack build src/index.html`

### `ATLASPACK_INCREMENTAL_BUNDLING`

Enable or disable incremental bundling.

- **Values**: `'true'` | `'false'`
- **Default**: `'true'`
- **Usage**: `ATLASPACK_INCREMENTAL_BUNDLING=false atlaspack build src/index.html`

## Build Environment

### `ATLASPACK_BUILD_ENV`

Sets the build environment context.

- **Values**: `'production'` | `'development'` | `'test'`
- **Default**: Based on command (production for build, development for serve/watch)
- **Usage**: `ATLASPACK_BUILD_ENV=test atlaspack build src/index.html`

### `ATLASPACK_SELF_BUILD`

Indicates Atlaspack is building itself (internal use).

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`

### `ATLASPACK_BUILD_REPL`

Enable REPL build mode for development.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`

## Debugging & Monitoring

Also see [`monitoring`](../../crates/atlaspack_monitoring/README.md)

### `ATLASPACK_SHOW_PHASE_TIMES`

Show detailed timing information for build phases.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_SHOW_PHASE_TIMES=true atlaspack build src/index.html`

### `ATLASPACK_TRACING_MODE`

Control tracing output for debugging.

- **Values**: `'stdout'` | (writes to `$TMPDIR/atlaspack_trace`)
- **Default**: Writes to temporary file
- **Usage**: `ATLASPACK_TRACING_MODE=stdout atlaspack build src/index.html`

### `ATLASPACK_DEBUG_CACHE_FILEPATH`

Debug cache file paths (testing only).

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_DEBUG_CACHE_FILEPATH=true atlaspack build src/index.html`

### `ATLASPACK_IDENTIFIER_DEBUG`

Enable identifier debugging and logging.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_IDENTIFIER_DEBUG=true atlaspack build src/index.html`

### `ATLASPACK_DUMP_GRAPHVIZ`

Enable GraphViz graph dumping for debugging. Also see [`BundlerExamples`](../../docs/BundlerExamples.md)

- **Values**: `'true'` | `'symbols'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_DUMP_GRAPHVIZ=symbols atlaspack build src/index.html`

### `ATLASPACK_ENABLE_SENTRY`

Enable Sentry error monitoring (canary releases only).

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`

### `ATLASPACK_SENTRY_DSN`

Sentry DSN for error reporting.

- **Usage**: `ATLASPACK_SENTRY_DSN=https://... atlaspack build src/index.html`

### `ATLASPACK_SENTRY_TAGS`

JSON string with tags for Sentry.

- **Usage**: `ATLASPACK_SENTRY_TAGS='{"env":"production"}' atlaspack build src/index.html`

### `ATLASPACK_ENABLE_MINIDUMPER`

Enable crash reporting with minidumps.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`

### `ATLASPACK_MINIDUMPER_SERVER_PID_FILE`

Path to PID file for minidumper server.

- **Usage**: `ATLASPACK_MINIDUMPER_SERVER_PID_FILE=/path/to/pid atlaspack build src/index.html`

### `ATLASPACK_MINIDUMPER_SERVER_SOCKET_NAME`

Socket path for minidumper server.

- **Usage**: `ATLASPACK_MINIDUMPER_SERVER_SOCKET_NAME=/path/to/socket atlaspack build src/index.html`

## Cache & Performance

### `ATLASPACK_DISABLE_CACHE_TIMEOUT`

Disable cache timeout for invalidation.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_DISABLE_CACHE_TIMEOUT=true atlaspack build src/index.html`

### `ATLASPACK_BYPASS_CACHE_INVALIDATION`

Bypass cache invalidation entirely.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_BYPASS_CACHE_INVALIDATION=true atlaspack build src/index.html`

## Development

### `ATLASPACK_DEV`

Development mode for APVM (requires APVM_ATLASPACK_LOCAL).

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_DEV=true atlaspack build src/index.html`

### `ATLASPACK_SHOULD_LOOK_FOR_EMPTY_FILES`

Control empty file detection in tests.

- **Values**: `'true'` | `'false'`
- **Default**: `'false'`
- **Usage**: `ATLASPACK_SHOULD_LOOK_FOR_EMPTY_FILES=true atlaspack build src/index.html`

## Usage Examples

### Performance Tuning

```bash
# Use more workers for faster builds
ATLASPACK_WORKERS=8 ATLASPACK_MAX_CONCURRENT_CALLS=50 atlaspack build src/index.html

# Configure native thread pools
ATLASPACK_NATIVE_THREADS=4 ATLASPACK_NAPI_WORKERS=8 atlaspack build src/index.html

# Disable incremental bundling for debugging
ATLASPACK_INCREMENTAL_BUNDLING=false atlaspack build src/index.html
```

### Debugging Builds

```bash
# Show detailed timing and trace to console
ATLASPACK_SHOW_PHASE_TIMES=true ATLASPACK_TRACING_MODE=stdout atlaspack build src/index.html

# Use process workers instead of threads
ATLASPACK_WORKER_BACKEND=process atlaspack build src/index.html

# Enable GraphViz debugging
ATLASPACK_DUMP_GRAPHVIZ=symbols atlaspack build src/index.html

# Debug cache and identifiers
ATLASPACK_DEBUG_CACHE_FILEPATH=true ATLASPACK_IDENTIFIER_DEBUG=true atlaspack build src/index.html
```

### Cache Management

```bash
# Disable cache timeout and bypass invalidation
ATLASPACK_DISABLE_CACHE_TIMEOUT=true ATLASPACK_BYPASS_CACHE_INVALIDATION=true atlaspack build src/index.html
```

### Production Monitoring

```bash
# Enable Sentry monitoring
ATLASPACK_ENABLE_SENTRY=true ATLASPACK_SENTRY_DSN=https://... atlaspack build src/index.html
```
