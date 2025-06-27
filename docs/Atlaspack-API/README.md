# Atlaspack Programmatic API Documentation

## Overview

Atlaspack provides a powerful programmatic API that allows you to integrate bundling, watching, and serving functionality directly into your Node.js applications. This documentation covers the available configuration options, API methods, and common use cases.

## Table of Contents

1. [Getting Started](#getting-started)
   - [Installation](#installation)
   - [Usage](#usage)
   - [Targets](#targets)
   - [Environment Variables](#environment-variables)
   - [Reporters](#reporters)
2. [API Methods](#api-methods)
   - [Core Methods](#core-methods)
   - [Development Server](#development-server)
   - [File System](#file-system)
   - [Advanced Methods](#advanced-methods)
   - [AtlaspackV3 Methods (Experimental)](#atlaspackv3-methods-experimental)

## Getting Started

### Installation

```bash
npm install @atlaspack/core
```

### Usage

```javascript
import Atlaspack from '@atlaspack/core';

const atlaspack = new Atlaspack({
  entries: 'src/index.html',
});
```

For a complete list of available configuration options, see the [Configuration Guide](../Configuration-Guide.md).

### Targets

By default, Atlaspack does a development build, but this can be changed by setting the `mode` option to `production`, which enables scope hoisting, minification, etc.

```javascript
const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  mode: 'production',
  defaultTargetOptions: {
    engines: {
      browsers: ['last 1 Chrome version'],
    },
  },
});
```

You can also use the `targets` option to specify which targets to build:

```javascript
// Build specific targets
const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  targets: ['modern'],
});

// Or define custom targets
const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  mode: 'production',
  targets: {
    modern: {
      engines: {
        browsers: ['last 1 Chrome version'],
      },
    },
    legacy: {
      engines: {
        browsers: ['IE 11'],
      },
    },
  },
});
```

### Environment Variables

Environment variables such as `NODE_ENV` may be set using the `env` option:

```javascript
const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  mode: 'production',
  env: {
    NODE_ENV: 'production',
  },
});
```

### Reporters

By default, Atlaspack does not write any output to the CLI when you use the API. You can enable CLI output using the `additionalReporters` option:

```javascript
import {fileURLToPath} from 'url';

const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  additionalReporters: [
    {
      packageName: '@atlaspack/reporter-cli',
      resolveFrom: fileURLToPath(import.meta.url),
    },
  ],
});
```

## API Methods

### Core Methods

#### `run()`

Builds the project and returns a `BuildSuccessEvent`.

```javascript
try {
  const {bundleGraph, buildTime} = await atlaspack.run();
  const bundles = bundleGraph.getBundles();
  console.log(`✨ Built ${bundles.length} bundles in ${buildTime}ms!`);
} catch (err) {
  console.log(err.diagnostics);
}
```

**Returns**: `Promise<BuildSuccessEvent>`

#### `watch(callback?)`

Starts watching for file changes and rebuilds automatically.

```javascript
const subscription = await atlaspack.watch((err, event) => {
  if (err) {
    // fatal error
    throw err;
  }

  if (event.type === 'buildSuccess') {
    const bundles = event.bundleGraph.getBundles();
    console.log(`✨ Built ${bundles.length} bundles in ${event.buildTime}ms!`);
  } else if (event.type === 'buildFailure') {
    console.log(event.diagnostics);
  }
});

// Stop watching
await subscription.unsubscribe();
```

**Parameters**:

- `callback`: `(err: Error | null, buildEvent?: BuildEvent) => void`

**Returns**: `Promise<AsyncSubscription>`

### Development Server

The development server is included in the default Atlaspack config. It can be enabled by passing `serveOptions` to the Atlaspack constructor and running Atlaspack in watch mode. Hot reloading can be enabled by setting `hmrOptions`:

```javascript
const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  serveOptions: {
    port: 3000,
  },
  hmrOptions: {
    port: 3000,
  },
});

await atlaspack.watch();
```

### File System

Atlaspack uses an abstracted file system throughout core and in most plugins. By default, it relies on the Node.js `fs` API, but Atlaspack also has a `MemoryFS` implementation. You can use the `inputFS` option to override the file system Atlaspack reads source files from, and the `outputFS` option to override the file system Atlaspack writes output (including the cache) to.

```javascript
import Atlaspack, {createWorkerFarm} from '@atlaspack/core';
import {MemoryFS} from '@atlaspack/fs';

const workerFarm = createWorkerFarm();
const outputFS = new MemoryFS(workerFarm);

const atlaspack = new Atlaspack({
  entries: 'src/index.html',
  workerFarm,
  outputFS,
});

try {
  const {bundleGraph} = await atlaspack.run();

  for (const bundle of bundleGraph.getBundles()) {
    console.log(bundle.filePath);
    console.log(await outputFS.readFile(bundle.filePath, 'utf8'));
  }
} finally {
  await workerFarm.end();
}
```

### Advanced Methods

#### `startProfiling()`

Starts performance profiling.

```javascript
await atlaspack.startProfiling();
```

#### `stopProfiling()`

Stops performance profiling and returns profile data.

```javascript
const profile = await atlaspack.stopProfiling();
```

#### `writeRequestTrackerToCache()`

Manually writes the request tracker to cache.

```javascript
await atlaspack.writeRequestTrackerToCache();
```

### AtlaspackV3 Methods (Experimental)

#### `buildAssetGraph()`

Builds the asset dependency graph.

```javascript
const graph = await atlaspackV3.buildAssetGraph();
```

#### `respondToFsEvents(events)`

Responds to file system events.

```javascript
const invalidated = atlaspackV3.respondToFsEvents([
  {path: '/path/to/file.js', type: 'update'},
]);
```
