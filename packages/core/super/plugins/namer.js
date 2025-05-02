let path = require('path');
let assert = require('assert');
let {Namer} = require('@atlaspack/plugin');

// This corresesponds to the worker entries in scripts/generate-entries.js
let workerEntries = {
  // Use the hash reference for the oringal worker file
  'core/workers/src/threads/ThreadsChild.js': (hashReference) =>
    `core/workers/threads/ThreadsChild.${hashReference}.js`,
  'core/workers/src/process/ProcessChild.js': (hashReference) =>
    `core/workers/process/ProcessChild.${hashReference}.js`,
  'core/core/src/worker.js': (hashReference) =>
    `core/core/worker.${hashReference}.js`,
  'core/core/src/atlaspack-v3/worker/napi-worker.js': (hashReference) =>
    `core/core/atlaspack-v3/worker/napi-worker.${hashReference}.js`,
  // Use a stable name for the entry worker files
  'core/super/entries/ThreadsChild.js': () =>
    `core/workers/threads/ThreadsChild.js`,
  'core/super/entries/ProcessChild.js': () =>
    `core/workers/process/ProcessChild.js`,
  'core/super/entries/worker.js': () => `core/core/worker.js`,
  'core/super/entries/napi-worker.js': () =>
    `core/core/atlaspack-v3/worker/napi-worker.js`,
};

module.exports = new Namer({
  name({bundle, options}) {
    let entryAsset = bundle.getMainEntry();

    if (bundle.type === 'node') {
      return path.join(
        'native',
        `${path.basename(entryAsset.filePath, '.node')}.${
          bundle.hashReference
        }.node`,
      );
    }

    if (entryAsset?.filePath.includes('/node_modules/')) {
      assert(
        !bundle.needsStableName,
        'Vendor bundles should not also be entries',
      );
      // This is the vendor bundle
      return `vendor.${bundle.hashReference}.${bundle.type}`;
    }

    let projectRelativePath = path.relative(
      path.join(options.projectRoot, '../..'),
      entryAsset.filePath,
    );

    if (projectRelativePath.startsWith('..')) {
      throw new Error(
        `Invalid relative path: "${projectRelativePath}" from "${options.projectRoot}"`,
      );
    }

    if (workerEntries[projectRelativePath]) {
      // Workers need to be positioned exactly as they are loaded via
      // path.join
      return workerEntries[projectRelativePath](bundle.hashReference);
    }

    if (projectRelativePath.endsWith('.json')) {
      // JSON files are actually compiled to JS files
      projectRelativePath = projectRelativePath + '.js';
    }

    if (bundle.needsStableName) {
      // This is an entry so just use it's base file name
      return path.basename(entryAsset.filePath);
    }

    // This must be another Atlaspack import so just name it to match it's
    // repo file path
    return projectRelativePath.replaceAll('/src/', '/');
  },
});
