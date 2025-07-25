# Atlaspack Configuration Guide

Atlaspack's programmatic API provides extensive configuration options to customize its behavior for your specific needs. This guide covers all available configuration options, from basic settings to advanced features, helping you optimize Atlaspack for your project requirements.

The configuration interface below shows all options available when creating an Atlaspack instance.

```typescript
interface InitialAtlaspackOptions {
  // Core configuration
  entries: FilePath | Array<FilePath>;

  // Basic configuration
  config?: DependencySpecifier;
  defaultConfig?: DependencySpecifier;
  env?: EnvMap;
  targets?: Array<string> | {[string]: TargetDescriptor};

  // Cache options
  shouldDisableCache?: boolean;
  cacheDir?: FilePath;

  // Project options
  projectRoot?: FilePath;
  gitRoot?: FilePath;

  // Watch options
  watchDir?: FilePath;
  watchBackend?: BackendType;
  watchIgnore?: Array<FilePath | GlobPattern>;

  // Build options
  mode?: BuildMode;
  shouldContentHash?: boolean;
  shouldBuildLazily?: boolean;
  lazyIncludes?: string[];
  lazyExcludes?: string[];
  shouldBundleIncrementally?: boolean;

  // Dev server options
  hmrOptions?: HMROptions | null;
  serveOptions?: InitialServerOptions | false;

  // System options
  inputFS?: FileSystem;
  outputFS?: FileSystem;
  cache?: Cache;
  workerFarm?: WorkerFarm;
  napiWorkerPool?: NapiWorkerPool;
  packageManager?: PackageManager;

  // Target options
  defaultTargetOptions?: {
    shouldOptimize?: boolean;
    shouldScopeHoist?: boolean;
    sourceMaps?: boolean;
    publicUrl?: string;
    distDir?: FilePath;
    engines?: Engines;
    outputFormat?: OutputFormat;
    isLibrary?: boolean;
  };

  // Other options
  shouldAutoInstall?: boolean;
  logLevel?: LogLevel;
  shouldProfile?: boolean;
  shouldTrace?: boolean;
  shouldPatchConsole?: boolean;
  additionalReporters?: Array<{
    packageName: DependencySpecifier;
    resolveFrom: FilePath;
  }>;
  featureFlags?: Partial<FeatureFlags>;
}
```

## Table of Contents

1. [Core Configuration](#core-configuration)
2. [Basic Configuration](#basic-configuration)
3. [Cache Configuration](#cache-configuration)
4. [Watch Configuration](#watch-configuration)
5. [Build Configuration](#build-configuration)
6. [Dev server Configuration](#dev-server-configuration)
7. [System Configuration](#system-configuration)
8. [Target Configuration](#target-configuration)
9. [Other Configuration](#other-configuration)

## Core Configuration

### `entries` (Required)

**Type**: `string | string[]`

**Description**: Entry points for your application. These are the files that Atlaspack will use as starting points for building your application.

**Examples**:

```javascript
// Single entry point
const atlaspack = new Atlaspack({
  entries: 'src/index.html',
});

// Multiple entry points
const atlaspack = new Atlaspack({
  entries: ['src/index.html', 'src/admin.html', 'src/dashboard.html'],
});
```

## Basic Configuration

### `config`

**Type**: `string`

**Description**: Path to a custom `.atlaspackrc` configuration file. This overrides the default configuration.

**Examples**:

```javascript
// Using a custom .atlaspackrc file
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  config: './custom.atlaspackrc',
});

// Using a JSON configuration file
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  config: './atlaspack.config.json',
});
```

### `defaultConfig`

**Type**: `string`

**Description**: Path to the default Atlaspack configuration. This is typically the `@atlaspack/config-default` package.

**Default**: `'@atlaspack/config-default'`

**Examples**:

```javascript
// Using a custom config
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  defaultConfig: './custom-config.js',
});
```

### `env`

**Type**: `object`

**Description**: Environment variables to pass to the build.

**Examples**:

```javascript
// Pass environment variables
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  env: {
    NODE_ENV: 'production',
    API_URL: 'https://api.example.com',
    DEBUG: 'false',
  },
});

// Merge with process.env
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  env: {
    ...process.env,
    NODE_ENV: 'production',
  },
});
```

### `targets`

**Type**: `string[] | object`

**Description**: Specific targets to build or custom Target Configurations.

**Examples**:

```javascript
// Custom Target Configurations
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  targets: {
    main: {
      source: 'src/main.js',
      distDir: 'dist/main',
      engines: {browsers: ['> 1%']},
      shouldOptimize: true,
    },
    legacy: {
      source: 'src/legacy.js',
      distDir: 'dist/legacy',
      engines: {browsers: ['ie 11']},
      shouldOptimize: false,
    },
  },
});
```

## Cache Configuration

### `shouldDisableCache`

**Type**: `boolean`

**Default**: `false`

**Description**: Disable caching during build.

**Examples**:

```javascript
// Disable cache for CI
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldDisableCache: process.env.CI === 'true',
});
```

### `cacheDir`

**Type**: `string`

**Default**: `'.parcel-cache'`

**Description**: Directory to store cache files.

**Examples**:

```javascript
// Custom cache directory
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  cacheDir: '.custom-cache',
});
```

## Watch Configuration

### `watchDir`

**Type**: `string`

**Default**: Project root

**Description**: Directory to watch for file changes.

**Examples**:

```javascript
// Watch specific directory
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  watchDir: 'src',
});
```

### `watchBackend`

**Type**: `'watchman' | 'fs-events' | 'inotify' | 'brute-force'`

**Description**: File watching backend to use.

**Examples**:

```javascript
// Use Watchman (if available)
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  watchBackend: 'watchman',
});
```

### `watchIgnore`

**Type**: `string[]`

**Description**: Patterns to ignore when watching files.

**Examples**:

```javascript
// Ignore specific patterns
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  watchIgnore: ['**/node_modules/**', '**/dist/**', '**/*.log'],
});
```

## Build Configuration

### `mode`

**Type**: `'development' | 'production' | string`

**Default**: `'development'`

**Description**: Build mode that affects optimization, source maps, and other settings.

**Examples**:

```javascript
// Development mode
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  mode: 'development',
});
```

### `shouldContentHash`

**Type**: `boolean`

**Default**: `true` in production, `false` in development

**Description**: Adds content hashes to bundle filenames for cache busting.

**Examples**:

```javascript
// Enable content hashing
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldContentHash: true,
});
```

### `shouldBuildLazily`

**Type**: `boolean`

**Default**: `false`

**Description**: Enables lazy building for large applications. Only builds assets that are actually requested.

**Examples**:

```javascript
// Enable lazy building
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldBuildLazily: true,
});
```

### `lazyIncludes` / `lazyExcludes`

**Type**: `string[]`

**Description**: Glob patterns to include/exclude from lazy building. Only used when `shouldBuildLazily` is `true`.

**Examples**:

```javascript
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldBuildLazily: true,
  lazyIncludes: ['src/components/**/*', 'src/pages/**/*'],
  lazyExcludes: ['src/components/legacy/**/*', 'src/pages/admin/**/*'],
});
```

### `shouldBundleIncrementally`

**Type**: `boolean`

**Default**: `true`

**Description**: Enables incremental bundling for faster rebuilds.

**Examples**:

```javascript
// Enable incremental bundling
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldBundleIncrementally: true,
});
```

## Dev server Configuration

### `hmrOptions`

**Type**: `object | null`

**Description**: Configuration for Hot Module Replacement (HMR).

**Properties**:

- `port`: HMR port
- `host`: HMR host

**Examples**:

```javascript
// Enable HMR
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  hmrOptions: {
    port: 3000,
    host: 'localhost',
  },
});

// Disable HMR
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  hmrOptions: null,
});
```

### `serveOptions`

**Type**: `object | false`

**Description**: Configuration for the development server.

**Properties**:

- `port`: Server port (default: 1234)
- `host`: Server host (default: 'localhost')
- `https`: HTTPS configuration (boolean or object with cert/key)
- `publicUrl`: Public URL for assets

**Examples**:

```javascript
// Basic development server
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  serveOptions: {
    port: 3000,
    host: 'localhost',
  },
});

// HTTPS development server
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  serveOptions: {
    port: 3000,
    host: 'localhost',
    https: {
      cert: './cert.pem',
      key: './key.pem',
    },
  },
});

// Disable built-in server
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  serveOptions: false,
});
```

## System Configuration

### `inputFS` / `outputFS`

**Type**: `FileSystem`

**Description**: Custom file system implementations for input and output.

**Examples**:

```javascript
import {MemoryFS} from '@atlaspack/fs';

// Use memory file system
const inputFS = new MemoryFS();
const outputFS = new MemoryFS();

const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  inputFS,
  outputFS,
});
```

### `cache`

**Type**: `Cache`

**Description**: Custom cache implementation.

**Examples**:

```javascript
import {FSCache} from '@atlaspack/cache';

// Use file system cache
const cache = new FSCache(outputFS, 'cache');

const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  cache,
});
```

### `workerFarm`

**Type**: `WorkerFarm`

**Description**: Custom worker farm implementation.

**Examples**:

```javascript
import {createWorkerFarm} from '@atlaspack/workers';

// Create custom worker farm
const workerFarm = createWorkerFarm({
  maxConcurrentWorkers: 4,
});

const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  workerFarm,
});
```

### `napiWorkerPool`

**Type**: `NapiWorkerPool`

**Description**: Custom NAPI worker pool implementation.

### `packageManager`

**Type**: `PackageManager`

**Description**: Custom package manager implementation.

**Examples**:

```javascript
import {NodePackageManager} from '@atlaspack/package-manager';

// Use Node package manager
const packageManager = new NodePackageManager(inputFS, '/');

const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  packageManager,
});
```

## Target Configuration

### `defaultTargetOptions`

**Type**: `object`

**Description**: Default options applied to all targets.

**Properties**:

- `shouldOptimize`: Enable optimization
- `shouldScopeHoist`: Enable scope hoisting
- `sourceMaps`: Generate source maps
- `publicUrl`: Public URL for assets
- `distDir`: Output directory
- `engines`: Target environments
- `outputFormat`: Output format
- `isLibrary`: Build as library

**Examples**:

```javascript
// Production target options
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  defaultTargetOptions: {
    shouldOptimize: true,
    shouldScopeHoist: true,
    sourceMaps: false,
    publicUrl: '/',
    distDir: 'dist',
    engines: {
      browsers: ['> 1%', 'last 2 versions'],
    },
  },
});

// Library target options
const atlaspack = new Atlaspack({
  entries: ['src/index.js'],
  defaultTargetOptions: {
    isLibrary: true,
    outputFormat: 'esmodule',
    shouldScopeHoist: true,
    shouldOptimize: true,
    publicUrl: './',
  },
});
```

## Other Configuration

### `shouldAutoInstall`

**Type**: `boolean`

**Default**: `true` in development, `false` in production

**Description**: Automatically install missing dependencies.

**Examples**:

```javascript
// Disable auto-install
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldAutoInstall: false,
});

// Environment-based auto-install
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldAutoInstall: process.env.NODE_ENV === 'development',
});
```

### `logLevel`

**Type**: `'none' | 'error' | 'warn' | 'info' | 'verbose'`

**Default**: `'info'`

**Description**: Controls the verbosity of Atlaspack's logging output.

**Examples**:

```javascript
// Silent mode (no logs)
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  logLevel: 'none',
});

// Environment-based logging
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  logLevel: process.env.VERBOSE ? 'verbose' : 'info',
});
```

### `shouldProfile`

**Type**: `boolean`

**Default**: `false`

**Description**: Enable performance profiling.

**Examples**:

```javascript
// Enable profiling
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldProfile: true,
});

// Environment-based profiling
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldProfile: process.env.PROFILE === 'true',
});
```

### `shouldTrace`

**Type**: `boolean`

**Default**: `false`

**Description**: Enable tracing for debugging.

**Examples**:

```javascript
// Enable tracing
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldTrace: true,
});
```

### `shouldPatchConsole`

**Type**: `boolean`

**Default**: `true`

**Description**: Patch console methods for better integration.

**Examples**:

```javascript
// Disable console patching
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  shouldPatchConsole: false,
});
```

### `additionalReporters`

**Type**: `Array<{packageName: string, resolveFrom: string}>`

**Description**: Additional reporters to use during the build process.

**Examples**:

```javascript
// Add custom reporters
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  additionalReporters: [
    {
      packageName: '@atlaspack/reporter-custom',
      resolveFrom: __dirname,
    },
  ],
});
```

### `featureFlags`

**Type**: `object`

**Description**: Enable/disable experimental features.

**Examples**:

```javascript
// Enable experimental features
const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  featureFlags: {
    atlaspackV3: true,
    cachePerformanceImprovements: true,
  },
});
```

## Configuration Best Practices

### 1. Environment-Based Configuration

```javascript
const isProduction = process.env.NODE_ENV === 'production';
const isCI = process.env.CI === 'true';

const atlaspack = new Atlaspack({
  entries: ['src/index.html'],
  mode: isProduction ? 'production' : 'development',
  shouldDisableCache: isCI,
  shouldContentHash: isProduction,
  logLevel: process.env.VERBOSE ? 'verbose' : 'info',
  env: {
    NODE_ENV: process.env.NODE_ENV || 'development',
  },
});
```
