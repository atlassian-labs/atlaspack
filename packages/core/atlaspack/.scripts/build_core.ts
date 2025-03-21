import * as fs from 'node:fs';
import glob from 'glob';
import {Paths} from './paths.ts';
import {writeFile, cp} from './fs.ts';
import {gatherDependencies} from './gather_deps.ts';

export async function buildCore() {
  // packages/core/diagnostic
  await cp(
    Paths['root/']('packages/core/diagnostic/lib'),
    Paths['~/']('diagnostic'),
  );
  await writeFile(
    Paths['~/']('diagnostic/index.js'),
    'module.exports = require("./diagnostic.js")',
  );
  await writeFile(
    Paths['~/']('diagnostic/index.d.ts'),
    'export * from "./diagnostic"',
  );

  // packages/core/feature-flags
  await cp(
    Paths['root/']('packages/core/feature-flags/lib'),
    Paths['~/']('feature-flags'),
  );
  await writeFile(
    Paths['~/']('feature-flags/index.d.ts'),
    'export * from "./types"',
  );

  // packages/utils/events
  await cp(Paths['root/']('packages/utils/events/lib'), Paths['~/']('events'));

  // packages/core/rust
  for (const match of [
    Paths['root/']('packages/core/rust/index.js'),
    Paths['root/']('packages/core/rust/index.d.ts'),
    Paths['root/']('packages/core/rust/browser.js'),
    ...glob.sync(Paths['root/']('packages/core/rust/*.node')),
    ...glob.sync(Paths['root/']('packages/core/rust/*.wasm')),
  ]) {
    let trimmed = match.replace(Paths['root/']('packages/core/rust'), '');
    await cp(match, Paths['~/']('rust', trimmed));
  }

  // packages/core/logger
  await cp(Paths['root/']('packages/core/logger/lib'), Paths['~/']('logger'));
  await writeFile(
    Paths['~/']('logger/index.js'),
    'module.exports = require("./Logger.js")',
  );
  await writeFile(Paths['~/']('logger/index.d.ts'), 'export * from "./Logger"');

  // packages/core/types-internal
  await cp(
    Paths['root/']('packages/core/types-internal/lib'),
    Paths['~/']('types-internal'),
  );
  await writeFile(
    Paths['~/']('types-internal/index.js'),
    'module.exports = {}\n',
  );

  // packages/core/codeframe
  await cp(
    Paths['root/']('packages/core/codeframe/lib'),
    Paths['~/']('codeframe'),
  );
  await writeFile(
    Paths['~/']('codeframe/index.js'),
    'module.exports = require("./codeframe.js")',
  );

  // packages/core/markdown-ansi
  await cp(
    Paths['root/']('packages/core/markdown-ansi/lib'),
    Paths['~/']('markdown-ansi'),
  );
  await writeFile(
    Paths['~/']('markdown-ansi/index.js'),
    'module.exports = require("./markdown-ansi.js")',
  );

  // packages/core/utils
  await cp(Paths['root/']('packages/core/utils/lib'), Paths['~/']('utils'));

  // packages/core/profiler
  await cp(
    Paths['root/']('packages/core/profiler/lib'),
    Paths['~/']('profiler'),
  );

  // packages/core/build-cache
  await cp(
    Paths['root/']('packages/core/build-cache/lib'),
    Paths['~/']('build-cache'),
  );

  // packages/utils/atlaspack-watcher-watchman-js
  await cp(
    Paths['root/']('packages/utils/atlaspack-watcher-watchman-js/lib'),
    Paths['~/']('atlaspack-watcher-watchman-js'),
  );

  // packages/core/workers
  await cp(Paths['root/']('packages/core/workers/lib'), Paths['~/']('workers'));

  // packages/core/types
  await cp(Paths['root/']('packages/core/types/lib'), Paths['~/']('types'));
  await writeFile(Paths['~/']('types/index.js'), 'module.exports = {}\n');

  // packages/core/fs
  await cp(Paths['root/']('packages/core/fs/lib'), Paths['~/']('fs'));
  await cp(
    Paths['root/']('packages/core/fs/index.d.ts'),
    Paths['~/']('fs/index.d.ts'),
  );

  // packages/utils/domain-sharding
  await cp(
    Paths['root/']('packages/utils/domain-sharding/lib'),
    Paths['~/']('domain-sharding'),
  );

  // packages/utils/ts-utils
  await cp(
    Paths['root/']('packages/utils/ts-utils/lib'),
    Paths['~/']('ts-utils'),
  );

  // packages/core/plugin
  await cp(Paths['root/']('packages/core/plugin/lib'), Paths['~/']('plugin'));
  await cp(
    Paths['root/']('packages/core/plugin/src/PluginAPI.d.ts'),
    Paths['~/']('plugin/PluginAPI.d.ts'),
  );

  await writeFile(
    Paths['~/']('plugin/index.js'),
    'module.exports = require("./PluginAPI.js")',
  );
  await writeFile(
    Paths['~/']('plugin/index.d.ts'),
    'export * from "./PluginAPI"',
  );

  // packages/utils/node-resolver-core
  await cp(
    Paths['root/']('packages/utils/node-resolver-core/lib'),
    Paths['~/']('node-resolver-core'),
  );

  // packages/core/cache
  await cp(Paths['root/']('packages/core/cache/lib'), Paths['~/']('cache'));
  await writeFile(Paths['~/']('cache/index.d.ts'), 'export * from "./types"');

  // packages/core/graph
  await cp(Paths['root/']('packages/core/graph/lib'), Paths['~/']('graph'));

  // packages/core/package-manager
  await cp(
    Paths['root/']('packages/core/package-manager/lib'),
    Paths['~/']('package-manager'),
  );
  await cp(
    Paths['root/']('packages/core/package-manager/index.d.ts'),
    Paths['~/']('package-manager/index.d.ts'),
  );

  // packages/core/core
  await cp(Paths['root/']('packages/core/core/lib'), Paths['~/']('core'));
  await cp(
    Paths['root/']('packages/core/core/index.d.ts'),
    Paths['~/']('core/index.d.ts'),
  );

  // packages/core/cli
  await cp(Paths['root/']('packages/core/cli/lib'), Paths['~/']('cli'));
  await writeFile(
    Paths['~/']('cli/index.js'),
    `#!/usr/bin/env node
    'use strict'; require('./cli');`,
  );
}
