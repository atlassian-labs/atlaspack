# Atlaspack Agent Guide

## Critical Rules ⚠️

**MUST READ FIRST - These rules are non-negotiable:**

1. **Public Repository Warning**: This is a **PUBLIC** repository. Never include internal/sensitive Atlassian information in commits, PRs, or code.

2. **File Operations**:
   - **MUST read files before editing them** - The edit tool will fail if you haven't read the file first
   - **ALWAYS prefer editing existing files** over creating new ones
   - **NEVER create documentation files** unless explicitly requested
   - Use specialized tools (read_files, edit, write) instead of bash commands for file operations where possible

3. **Communication Style**:
   - **Do not use emoji** unless explicitly requested by the user
   - Keep responses concise and technical
   - Output text directly to user, NOT via bash echo or comments

4. **Code Quality**:
   - **NEVER use placeholders** in code or function calls - always use real values or ask for them
   - **ALWAYS run appropriate tests** for the area you're modifying
   - **Format files after editing** using the format_files tool
   - **Prefer writing or improving tests when investigating unexpected behaviour** instead of just reading the code or using CLI.

5. **Git Safety**:
   - **NEVER use interactive git commands** (`-i` flag) - not supported in automation
   - NEVER skip hooks (--no-verify, --no-gpg-sign)
   - NEVER update git config
   - **NEVER push or commit without explicit confirmation**
   - **NEVER run destructive git commands** (force push, hard reset) unless explicitly requested
   - NEVER force push to main/master
   - **NEVER create PRs without asking first** and getting developer approval of the description

6. **Task Management**:
   - **Use the todo tool proactively** for complex multi-step tasks
   - Mark todos complete immediately after finishing each step
   - Only one task should be in_progress at a time

7. **When stuck**:
   - Check "Development Workflow Guide" section
   - Check "Common Pitfalls" section
   - Run `yarn clean-test` if cache issues
   - Check CI logs for similar failures
   - Review recent commits for context
   - Ask the developer for clarification

## Project Overview

Atlaspack is a high-performance frontend bundler designed to build exceptionally large applications at Atlassian scale. It is written in JavaScript/TypeScript and Rust, forked from Parcel, and optimized for internal Atlassian product development. While publicly available, it is not intended for production use outside Atlassian.

### Core Architecture

Atlaspack follows a **plugin-based architecture** with these key components:

1. **Core Engine** - Orchestrates the build process through request tracking and caching
2. **Asset Graph** - Dependency graph that tracks all assets and their relationships
3. **Bundle Graph** - Determines how assets are grouped into output bundles
4. **Plugin System** - Transformers, Resolvers, Bundlers, Namers, Optimizers, Packagers, Reporters, and Compressors

### File Structure

```
packages/                             # JavaScript/TypeScript packages
├── core/                             # Core Atlaspack packages
├── transformers/                     # Transformer plugins (JS, CSS, HTML, etc.)
├── bundlers/                         # Bundler plugins
├── optimizers/                       # Optimizer plugins (minifiers, etc.)
├── packagers/                        # Packager plugins
├── resolvers/                        # Module resolvers
├── namers/                           # Bundle naming strategies
├── reporters/                        # Build reporters (CLI, dev server, etc.)
├── runtimes/                         # Runtime code injected into bundles
├── utils/                            # Shared utilities
├── dev/                              # Development tools
│   ├── atlaspack-inspector/          # Build inspector UI
│   ├── query/                        # Query tool for builds
│   └── bundle-stats-cli/             # Bundle statistics
└── examples/                         # Example projects for testing
crates/                               # Rust crates
├── atlaspack_plugin_transformer_js/  # SWC-based JS transformer
├── atlaspack_plugin_resolver/        # Module resolver
├── atlaspack_filesystem/             # FS operations
├── atlaspack_sourcemap/              # Source map handling
├── atlaspack_monitoring/             # Sentry integration
├── lmdb-js-lite/                     # LMDB bindings
└── ...
docs/                                 # Documentation
.github/                              # GitHub Actions CI
└── workflows/ci.yml                  # Main CI configuration
scripts/                              # Build and utility scripts
```

### Language Split: JavaScript/TypeScript and Rust

Atlaspack is a **hybrid codebase**:

- **JavaScript/TypeScript**:
  - Core orchestration
  - Plugin coordination
  - Configuration
  - Some plugin implementations
- **Rust**: Performance-critical operations including:
  - JavaScript transformation (SWC-based, in `packages/transformers/js/core/`)
  - CSS transformation (Lightning CSS)
  - HTML parsing and transformation
  - Image optimization
  - Resolver logic
  - Native LMDB bindings for caching

Atlaspack aims to eventually become a fully native Rust bundler.

### Key Differences from Webpack and Babel

**vs Webpack:**

- Zero-config by default with sensible defaults
- Faster due to Rust-based transformers and parallel processing
- Built-in dev server with HMR
- Asset graph-based rather than purely module-based
- Native support for scope hoisting and tree shaking

**vs Babel:**

- Atlaspack uses SWC (Rust-based) instead of Babel for JavaScript transformation by default
- Babel plugin available as `@atlaspack/transformer-babel` for compatibility
- Direct AST manipulation in Rust for better performance
- Integrated build pipeline rather than just a transpiler
- More rigid, structured and fine-grained APIs

### Build Modes

- **Development**: Fast rebuilds, no scope hoisting, includes debugging info
- **Production**: Scope hoisting enabled, minification, tree shaking, optimizations

## Development Workflow Guide

This section consolidates all workflow commands and patterns for development. Refer to this section for any build, test, or deployment workflow questions.

### Daily Development Workflow

**When to rebuild:**

```bash
# After pulling or making changes to Rust code:
yarn build-native

# After pulling or making changes to JS/TS code:
yarn build

# For active development (watch mode - rebuilds on file changes):
./scripts/dev

# Clean rebuild from scratch:
yarn clean
yarn build-native-release
yarn build

# If tests are failing unexpectedly:
yarn clean-test
```

### Build Commands Reference

**Native (Rust) artifacts:**

```bash
yarn build-native              # Development build (faster)
yarn build-native-release      # Release build (optimized, slower)
yarn build-native-wasm         # WASM build
```

Build artifacts are stored in `packages/` as platform-specific native modules (`.node` files).

**JavaScript/TypeScript:**

```bash
yarn build                     # Build everything (clean, prepare, gulp, TypeScript)
yarn build:ts                  # Type check TypeScript only
```

### Testing Workflow

**Choose tests based on what you're modifying:**

```bash
# Unit tests (fast - use during active development):
yarn test:js:unit              # JS/TS unit tests only
cargo test                     # Rust unit tests only
yarn test:unit                 # Both JS and Rust unit tests

# Integration tests (slower - use before committing):
yarn test:integration          # Full integration test suite
yarn test:integration-ci       # CI mode (includes retries)
yarn test:integration:v3       # V3 experimental features

# E2E tests (slowest - use for user-facing changes):
yarn test:e2e                  # End-to-end tests with real builds

# All tests (very slow ~30+ minutes - use before creating PR):
yarn test                      # Runs unit + integration

# Specific package tests:
yarn workspace @atlaspack/integration-tests test
yarn workspace @atlaspack/inspector test:unit
yarn workspace @atlaspack/inspector test:e2e

# Run a single test:
yarn test:js:unit --grep "test name pattern"

# Clean test cache if tests behave unexpectedly:
yarn clean-test
```

**When tests fail:**

1. Read the error message carefully - Atlaspack has good error messages
2. Check if you need to fully rebuild: `yarn build-native && yarn build`
3. Clean cache if tests behave unexpectedly: `yarn clean-test`
4. Use `ATLASPACK_MOCHA_HANG_DEBUG=true` for hanging tests
5. Check recent commits - tests may be flaky, compare with CI results
6. Keep task as `in_progress`, don't mark completed
7. Create new task describing what needs resolution
8. Fix the issue before moving on

### Linting and Formatting Workflow

**Before completing any task:**

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

**After editing files:** Use the `format_files` tool to format them.

## Repository Information

### Package Manager and Monorepo

- **Package Manager**: Yarn
- **Monorepo Tool**: Lerna for managing multiple packages
- **Workspaces**: Located in:
  - `packages/*/*` - Main packages (core, transformers, bundlers, etc.)
  - `crates/*` - Rust crates
  - `benchmarks/*`
  - `packages/examples/*` - Example projects for testing
  - `scripts/` - Build and utility scripts

### Prerequisites

- Node.js LTS (>= 16.0.0, recommend v20+)
- Rust stable toolchain
- Yarn v1 (v1.22.19)

## Git and CI

CI configuration: `.github/workflows/ci.yml`

### Git Workflow

- Main branch: `main`
- **Do not push to the repository without confirmation**
- **Never use interactive git commands** (`-i` flag) - not supported in automation
- Commit messages should be concise and focus on "why" rather than "what"
- Avoid `git commit --amend` unless explicitly requested or fixing pre-commit hook issues
- Always check authorship before amending: `git log -1 --format='%an %ae'`

### Pull Requests

- Use the template in `.github/PULL_REQUEST_TEMPLATE.md`
- Each PR requires a **changeset** via `yarn changeset` if it affects published packages
  - Add to PR description: `<!-- [no-changeset]: Explanation why -->` if no changeset needed
- This is a **public repository** - no internal/sensitive information in commits or PRs
- **Always ask before creating PRs** and ensure the developer reviews the description
- Create PRs using GitHub CLI: `gh pr create`
- **Never push without confirmation**

## Project Architecture

### Plugin System Overview

Atlaspack uses a **plugin-based architecture** where transformations happen through specialized plugins:

- **Transformers**: Convert source files to Atlaspack-compatible format
- **Resolvers**: Find dependencies and resolve import paths
- **Bundlers**: Determine how assets are grouped into bundles
- **Namers**: Generate output filenames
- **Packagers**: Concatenate assets into final bundle files
- **Optimizers**: Minify and optimize bundled code
- **Reporters**: Report build progress and results
- **Compressors**: Compress output files

### Transformers (Condensed)

Transformers implement the `Transformer` interface from `@atlaspack/plugin`. Key concepts:

**Basic Structure:**

```typescript
import {Transformer} from '@atlaspack/plugin';

export default new Transformer({
  async transform({asset, config, options, logger}) {
    let code = await asset.getCode();
    let result = transformCode(code);
    asset.setCode(result.code);
    asset.addDependency({specifier: './module', specifierType: 'esm'});
    return [asset];
  },
});
```

**Key Asset Methods:**

- `asset.getCode()` / `asset.setCode(code)` - Read/write content
- `asset.addDependency(dep)` - Declare dependencies
- `asset.invalidateOnFileChange(path)` - Cache invalidation
- `asset.meta` - Store metadata

**JavaScript Transformer** (`packages/transformers/js/`):

- Most complex transformer, combines TypeScript orchestration with Rust (SWC-based) transformation
- Handles: TypeScript, JSX, dependency collection, scope hoisting, tree shaking
- Pipeline: Parse → SWC visitors → Atlaspack transforms (env vars, globals, fs inlining) → Code generation
- In production: Enables scope hoisting (concatenates modules into single scope like Rollup)
- Location: `src/JSTransformer.ts` (orchestration), `core/src/` (Rust implementation)

**Key Differences from Babel/Webpack:**

- Babel: Sequential AST transforms, JS-based, transpiler only
- Webpack loaders: Sequential chain, single input→output
- Atlaspack: Parallel where possible, Rust-based (faster), can return multiple assets, integrated caching

**Execution Flow:**

1. Core requests asset transformation
2. Config selects transformer based on file type
3. Transformer runs (in worker pool if multi-threaded)
4. Results cached in LMDB
5. Asset graph updated with transformed assets and dependencies

**Notes:**

- Atlaspack transformers are often orchestrated with other transformers in Babel and the like. In such cases the project's native transformers typically run after external transformers.

## Important Subsystems

### Scope Hoisting and Tree Shaking

**Symbols**: Atlaspack tracks imported and exported symbols for each asset.

**Tree Shaking Process**:

1. **Symbol Collection**: During transformation, collect all exports
2. **Symbol Propagation**: Determine which symbols are actually used
3. **Dead Code Elimination**: Remove unused exports and their dependencies
4. **Scope Hoisting**: Concatenate modules into single scope in production

**Deferring**: Assets can be "deferred" (not transformed) if their exports aren't used. This speeds up builds significantly for large libraries where only a subset is imported.

See `docs/Scopehoisting.md` and `docs/Symbol Propagation.md` for details.

### Conditional Bundling

Atlaspack supports **conditional imports** for platform-specific code:

```javascript
import {foo} from 'conditional:./platform';
```

This allows bundling different code based on build conditions (e.g., browser vs. Node.js). Configuration lives in `@atlaspack/conditional-import-types`.

See `docs/features/Conditional Bundling.md` for implementation details.

### Caching System

- **LMDB-based**: Fast key-value store (Rust bindings in `crates/lmdb-js-lite/`)
- **Request Tracker**: Tracks all build requests and dependencies
- **Invalidation**: Based on file changes, config changes, and plugin changes
- **Location**: `.parcel-cache/` in project root (usually gitignored)

### Monitoring and Crash Reporting

**Sentry Integration** (`crates/atlaspack_monitoring/`):

- Crash reporting for Rust panics
- Performance tracing
- Configured via environment variables for Atlassian products

**Tracing**:

- Chrome tracing format support
- Enable with profiler API or reporter

### Rust Architecture

**Key Crates**:

- `atlaspack_core` - Core types and asset graph
- `atlaspack_plugin_transformer_js` - JS transformer plugin (SWC-based)
- `atlaspack_plugin_resolver` - Module resolution
- `atlaspack_filesystem` - Abstracted FS operations
- `atlaspack_sourcemap` - Source map handling
- `atlaspack_monitoring` - Sentry integration for crash reporting
- `lmdb-js-lite` - LMDB bindings for caching
- `node-bindings` - N-API bindings for Node.js

**Testing Rust Code**:

- Tests use `#[cfg(test)]` attribute in same file as implementation
- Run with `cargo test` or `cargo test -- --nocapture` to see stdout
- No separate test directories for Rust
- Use `dbg!()` macro for debugging

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

**Domain Sharding** (`packages/utils/domain-sharding/`):

- Utilities for distributing assets across multiple domains
- Performance optimization for large applications

**VCS Caching**:

- Atlaspack can cache based on git commits
- Speeds up CI builds by reusing cache from previous commits

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

## Environment Variables

### Core Build Variables

- `NODE_ENV` - Set to `production` or `development`
- `ATLASPACK_BUILD_ENV` - Build environment (`production`, `test`)
- `ATLASPACK_REGISTER_USE_SRC` - Use source files instead of built files
- `CARGO_PROFILE` - Rust build profile (`dev`, `release`)
- `RUSTUP_TARGET` - Rust compilation target

### Feature Flags & Experimental

- `ATLASPACK_V3` - Enable V3 experimental features

### Testing & Debugging

- `ATLASPACK_MOCHA_HANG_DEBUG=true` - Debug hanging Mocha tests with `why-is-node-running`
- `NODE_OPTIONS='--inspect-brk'` - Enable Node.js debugging with Chrome DevTools
- `RUST_BACKTRACE=full` - Full Rust backtraces (used in CI)

### Performance & Caching

- `ATLASPACK_WORKERS` - Control number of worker threads
- `SCCACHE_GHA_ENABLED` - Enable sccache in GitHub Actions (CI only)
- `RUSTC_WRAPPER=sccache` - Use sccache for Rust compilation caching (CI only)

### CI-Specific

- `GITHUB_TOKEN` - GitHub personal access token for releases (needs `read:user` and `read:repo`)
- `SENTRY_*` - Sentry configuration for crash reporting in production builds
