# Atlaspack Agent Guide

## Critical Rules ⚠️

1. **Public Repository Warning**: This is a PUBLIC repository. Never include internal/sensitive Atlassian information in the project.

2. **File Operations**:
   - Always read files before editing them
   - Always prefer editing existing files instead of creating new ones
   - Always use specialized tools (read_files, edit, write) instead of bash commands where possible
   - Never create documentation files unless directed

3. **Communication Style**:
   - Always be concise and technical
   - Always output text directly to user, NOT via bash echo or comments
   - Always request more information when necessary
   - Never use emoji

4. **Code Quality**:
   - Always run relevant tests for the area of code being modified
   - Always improve the test suite when fixing issues instead of just reading the code or using CLI
   - Always format files after editing using the relevant tool
   - Never use placeholders in code, always use real values or ask for them

5. **Git Safety**:
   - Never push or commit unless directed
   - Never update git config
   - Never force push to main/master
   - Never use interactive git commands (`-i` flag)
   - Never skip hooks (--no-verify, --no-gpg-sign)
   - Never run destructive git commands unless directed
   - Never create PRs unless directed

6. **Task Management**:
   - Always use the todo/task tool
   - Always mark todos/tasks as complete immediately after finishing them
   - Never work on multiple todos/tasks simultaneously

7. **When stuck**:
   - Check "Development Workflow Guide" section
   - Ask the developer for clarification
   - Review recent commits for context

## Project Overview

Atlaspack is a high-performance frontend bundler designed to build exceptionally large applications at Atlassian scale. It is written in JavaScript/TypeScript and Rust, forked from Parcel, and optimized for internal Atlassian product development. While publicly available, it is not intended for production use outside Atlassian.

### Core Architecture

Atlaspack follows a plugin-based architecture with these key components:

1. **Core Engine** - Orchestrates the build process through request tracking and caching
2. **Asset Graph** - Dependency graph that tracks all assets and their relationships
3. **Bundle Graph** - Determines how assets are grouped into output bundles
4. **Plugin System** - Specialised and modular extensions
   - **Transformers**: Convert source files to Atlaspack-compatible format
   - **Resolvers**: Find dependencies and resolve import paths
   - **Bundlers**: Determine how assets are grouped into bundles
   - **Namers**: Generate output filenames
   - **Packagers**: Concatenate assets into final bundle files
   - **Optimizers**: Minify and optimize bundled code
   - **Reporters**: Report build progress and results
   - **Compressors**: Compress output files

### Project Structure

- Atlaspack uses the `yarn` package manager for JS/TS packages
- Atlaspack uses Lerna for managing its multiple JS/TS packages
- Atlaspack uses a Cargo workspace for Rust crates (defined in root `Cargo.toml`)

```
packages/                                # JavaScript/TypeScript packages
├── core/                                # Core Atlaspack packages
├── transformers/                        # Transformer plugins (JS, CSS, HTML, etc.)
├── bundlers/                            # Bundler plugins
├── optimizers/                          # Optimizer plugins (minifiers, etc.)
├── packagers/                           # Packager plugins
├── resolvers/                           # Module resolvers
├── namers/                              # Bundle naming strategies
├── reporters/                           # Build reporters (CLI, dev server, etc.)
├── runtimes/                            # Runtime code injected into bundles
├── utils/                               # Shared utilities
├── dev/                                 # Development tools
│   ├── atlaspack-inspector/             # Build inspector UI
│   ├── query/                           # Query tool for builds
│   └── bundle-stats-cli/                # Bundle statistics
└── examples/                            # Example projects for testing
crates/                                  # Rust crates (Cargo workspace)
├── atlaspack/                           # Main Atlaspack crate
├── atlaspack_core/                      # Core types and asset graph
├── atlaspack_config/                    # Configuration handling
├── atlaspack_filesystem/                # FS operations
├── atlaspack_sourcemap/                 # Source map handling
├── atlaspack_monitoring/                # Sentry integration for crash reporting
├── atlaspack_plugin_transformer_js/     # SWC-based JS transformer
├── atlaspack_plugin_transformer_css/    # Lightning CSS transformer
├── atlaspack_plugin_transformer_html/   # HTML parser/transformer
├── atlaspack_plugin_transformer_image/  # Image optimization
├── atlaspack_plugin_resolver/           # Module resolver
├── atlaspack_plugin_rpc/                # Plugin RPC communication
├── atlaspack_swc_runner/                # SWC runner utilities
├── lmdb-js-lite/                        # LMDB bindings for caching
├── node-bindings/                       # N-API bindings for Node.js
└── ...                                  # Other crates (VCS, macros, etc.)
docs/                                    # Documentation
.github/                                 # GitHub Actions CI
└── workflows/ci.yml                     # Main CI configuration
scripts/                                 # Build and utility scripts
```

### Language Split: JavaScript/TypeScript and Rust

Atlaspack is a **hybrid codebase**:

- **JavaScript/TypeScript**:
  - Core orchestration
  - Plugin coordination
  - Configuration
  - Some plugin implementations
- Rust: Performance-critical operations including:
  - Core orchestration (feature gated)
  - JavaScript transformation (SWC-based, in `packages/transformers/js/core/`)
  - CSS transformation (Lightning CSS)
  - HTML parsing and transformation
  - Image optimization
  - Resolver logic
  - Native LMDB bindings for caching

Atlaspack aims to eventually become a fully native Rust bundler.

### Build Modes

- **Development**: Fast rebuilds, no scope hoisting, includes debugging info
- **Production**: Scope hoisting enabled, minification, tree shaking, optimizations

## Development Workflow Guide

This section consolidates all workflow commands and patterns for development. Refer to this section for any build, test, or deployment workflow questions.

### Building

When to rebuild:

- Full rebuild required for using Atlaspack for actual local bundling after any code changes
- Native rebuild required after altering Rust code
- JS build not required for running integration tests

Native (Rust) artifacts:

```bash
yarn build-native              # Development build (faster)
yarn build-native-release      # Release build (optimized, slower)
yarn build-native-wasm         # WASM build
```

Build artifacts are stored in `packages/` as platform-specific native modules (`.node` files) and used by the JavaScript runtime.

JavaScript/TypeScript:

```bash
yarn build                     # Build everything (clean, prepare, gulp, TypeScript)
yarn build:ts                  # Type check TypeScript only
```

### Testing

- Tests should be run for the modified region
- Always run tests from project root
- `cargo nextest` should be used instead of `cargo test` if installed

```bash
# Unit tests (fast - use during active development):
yarn test:js:unit              # All JS/TS unit tests
cargo test                     # All Rust unit tests
cargo test -p atlaspack_core   # Specific Rust package tests
yarn test:unit                 # All unit tests

# Integration tests (slower - only run affected area):
yarn test:integration          # Full integration test suite
yarn test:integration-ci       # CI mode (includes retries)
yarn test:integration:v3       # V3 experimental features

# E2E tests (slowest - only use in CI):
yarn test:e2e                  # End-to-end tests with real builds

# All tests (extremely slow - only use in CI):
yarn test                      # Runs unit + integration

# Specific package tests:
yarn workspace @atlaspack/integration-tests test
yarn workspace @atlaspack/inspector test:unit
yarn workspace @atlaspack/inspector test:e2e

# Run a single test:
yarn test:js:unit --grep "test name pattern"     # JS unit test
yarn test:integration --grep "test name pattern" # JS integration test
cargo test test_name                             # Rust unit test (all packages)
cargo test -p atlaspack_core test_name           # Rust unit test (specific package - check Cargo.toml for exact name)
cargo test --lib module::tests::test_name        # Rust unit test (by module path)
cargo test --test integration_test_name          # Rust integration test

# Clean test cache if tests behave unexpectedly:
yarn clean-test
```

**When tests fail:**

1. Read the error message carefully
2. Check if you need to fully rebuild: `yarn build-native && yarn build`
3. Clean cache if tests behave unexpectedly: `yarn clean-test`
4. Use `ATLASPACK_MOCHA_HANG_DEBUG=true` for hanging tests
5. Check recent commits - tests may be flaky
6. Do not mark task as completed
7. Create new task describing what needs resolution
8. Fix the issue before moving on

### Linting and Formatting Workflow

Perform file linting and formatting before completing any task, using tools and/or the following commands:

```bash
# Lint all code:
yarn lint                      # Runs ESLint and Prettier

# Format all code:
yarn format                    # Format JS + Rust

# Rust linting:
cargo fmt --all -- --check     # Check Rust formatting
cargo clippy -- -D warnings    # Rust linting

# Other checks:
yarn build:ts                  # Type check TypeScript
yarn lint:feature-flags        # Check for unused feature flags
```

## Git and CI

CI configuration: `.github/workflows/ci.yml`

### Git Workflow

- Main branch: `main`
- Always write concise git messages that focus on "why" not "what"
- Never push to the repository unless directed
- Never use interactive git commands (`-i` flag)
- Never commit 'amended' unless directed

### Pull Requests

- Never create a PR unless directed
- Always get developer approval of the PR title and description
- Always use the template in `.github/PULL_REQUEST_TEMPLATE.md`
- Each PR requires a **changeset** via `yarn changeset` if it affects published packages
  - Add to PR description: `<!-- [no-changeset]: Explanation why -->` if no changeset needed
- This is a **public repository** - Never include internal/sensitive information **anywhere**
- Create PRs using GitHub CLI: `gh pr create`

## Project Systems

### Transformers

Transformers are given code and output modified code.

**Execution flow**:

1. Core requests asset transformation
2. Config selects transformer based on file type
3. Transformer runs (in worker pool if multi-threaded)
4. Results cached in LMDB
5. Asset graph updated with transformed assets and dependencies

**JavaScript Transformer** (`packages/transformers/js/`):

- Most complex transformer, combines TypeScript orchestration with Rust (SWC-based) transformation
- Handles: TypeScript, JSX, dependency collection, scope hoisting, tree shaking
- Pipeline: Parse → SWC visitors → Atlaspack transforms (env vars, globals, fs inlining) → Code generation
- In production: Enables scope hoisting (concatenates modules into single scope like Rollup)
- Location: `src/JSTransformer.ts` (orchestration), `core/src/` (Rust implementation)

**vs Babel/Webpack**:

- Babel: Sequential AST transforms, JS-based, transpiler only
- Webpack loaders: Sequential chain, single input→output
- Atlaspack: Parallel where possible, Rust-based (faster), can return multiple assets, integrated caching

**Notes**:

- Atlaspack transformers are often orchestrated with other transformers in Babel and the like. In such cases the project's native transformers typically run after external transformers.

### Scope Hoisting and Tree Shaking

Symbols: Atlaspack tracks imported and exported symbols for each asset.

Tree shaking process:

1. **Symbol Collection**: During transformation, collect all exports
2. **Symbol Propagation**: Determine which symbols are actually used
3. **Dead Code Elimination**: Remove unused exports and their dependencies
4. **Scope Hoisting**: Concatenate modules into single scope in production

Deferring: Assets can be "deferred" (not transformed) if their exports aren't used. This speeds up builds significantly for large libraries where only a subset is imported.

See `docs/Scopehoisting.md` and `docs/Symbol Propagation.md` for details.

### Conditional Bundling

Atlaspack supports **conditional imports** executed at **runtime**:

```javascript
import { foo } from importCond('cond', './a', './b');
```

Configuration lives in `@atlaspack/conditional-import-types`.

See `docs/features/Conditional Bundling.md` for implementation details.

### Caching System

- **LMDB-based**: Fast key-value store (Rust bindings in `crates/lmdb-js-lite/`)
- **Request Tracker**: Tracks all build requests and dependencies
- **Invalidation**: Based on file changes, config changes, and plugin changes
- **Location**: `.parcel-cache/` in project root (usually gitignored)

### Monitoring and Crash Reporting

Sentry Integration (`crates/atlaspack_monitoring/`):

- Crash reporting for Rust panics
- Performance tracing
- Configured via environment variables for Atlassian products

Tracing:

- Chrome tracing format support
- Enable with profiler API or reporter

### Additional Tools & Utilities

**Inspector** (`packages/dev/atlaspack-inspector/`):

- Debug tool for inspecting builds, bundle graphs, and cache
- Web-based UI for visualizing the build process
- Useful for understanding why certain bundles were created

**Examples** (`packages/examples/`):

- `kitchen-sink/` - Comprehensive example with many features
- Use these to test changes: `cd packages/examples/kitchen-sink && yarn start`

**Query Tool** (`packages/dev/query/`):

- CLI tool for querying build information
- Useful for inspecting deep imports and dependency trees

## Feature Flags

Atlaspack uses runtime and compile-time feature flags for gradual rollouts:

**Location**: `@atlaspack/feature-flags` package

**Usage**:

```typescript
import {getFeatureFlag} from '@atlaspack/feature-flags';

if (getFeatureFlag('myNewFeature')) {
  // New code path
}
```

**Linting**: Run `yarn lint:feature-flags` to find unused flags.
