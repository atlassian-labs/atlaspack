# Build Commands

The Atlaspack CLI is the most common way to use Atlaspack. It supports three different commands: `serve`, `watch`, and `build`.

## Common Options

These options are available for `atlaspack serve`, `atlaspack watch`, and `atlaspack build`:

- `--public-url <url>` - The path prefix for absolute URLs
- `--no-cache` - Disable the filesystem cache
- `--watch-ignore [path]` - List of directories watcher should not track for changes (default: ['.git', '.hg'])
- `--watch-backend <name>` - Set watcher backend (choices: watchman, fs-events, inotify, brute-force, windows)
- `--no-source-maps` - Disable sourcemaps
- `--target [name]` - Only build given target(s)
- `--log-level <level>` - Set the log level (choices: none, error, warn, info, verbose)
- `--dist-dir <dir>` - Output directory when unspecified by targets
- `--no-autoinstall` - Disable autoinstall
- `--profile` - Enable sampling build profiling
- `--profile-native [instruments|samply]` - Enable native build profiling (instruments on macOS, samply otherwise)
- `--trace` - Enable build tracing
- `-V, --version` - Output the version number
- `--detailed-report [count]` - Print asset timings and sizes in the build report
- `--reporter <name>` - Additional reporters to run
- `--feature-flag <name=value>` - Set the value of a feature flag

## `atlaspack serve [input...]`

Starts a development server, which will automatically rebuild your app as you change files, and supports hot reloading.

**Available Options:**

1. Common Build Options (see above)
2. Command-Specific Options:
   - `--open [browser]` - Automatically open in specified browser (defaults to default browser)
   - `--watch-for-stdin` - Exit when stdin closes
   - `--lazy [includes]` - Build async bundles on demand when requested in the browser. Defaults to all async bundles unless a comma-separated list of source file globs is provided
   - `--lazy-exclude <excludes>` - Can only be used with `--lazy`. Comma-separated list of source file globs to exclude from lazy building
   - `--production` - Run with production mode defaults

**Example:**

```bash
atlaspack serve src/index.html --open chrome --log-level verbose --no-cache --lazy "src/async/*" --public-url /app/
```

## `atlaspack watch [input...]`

Starts the bundler in watch mode. The watch command is similar to serve, but does not start a dev server (only a HMR server). However, it automatically rebuilds your app as you make changes, and supports hot reloading. Use watch if you're building a library, a backend, or have your own dev (HTTP) server.

**Available Options:**

1. Common Build Options (see above)
2. Command-Specific Options:
   - `--no-content-hash` - Disable content hashing
   - `--watch-for-stdin` - Exit when stdin closes
   - `--production` - Run with production mode defaults

**Example:**

```bash
atlaspack watch src/index.html --no-content-hash --log-level info --watch-ignore "node_modules" --public-url /app/
```

## `atlaspack build [input...]`

Bundles for production. The build command performs a single production build and exits. This enables scope hoisting and other production optimizations by default.

**Available Options:**

1. Common Build Options (see above)
2. Command-Specific Options:
   - `--no-optimize` - Disable minification
   - `--no-scope-hoist` - Disable scope-hoisting
   - `--no-content-hash` - Disable content hashing

**Example:**

```bash
atlaspack build src/index.html --no-optimize --dist-dir ./dist --log-level verbose --public-url /app/
```

## Getting Help

- Use `--help` or `-h` with any command to see its specific options and usage
- Example: `atlaspack serve --help`
