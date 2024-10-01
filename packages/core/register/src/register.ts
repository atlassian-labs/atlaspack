import type {IDisposable, InitialAtlaspackOptions} from '@atlaspack/types';

import {NodePackageManager} from '@atlaspack/package-manager';
import {NodeFS} from '@atlaspack/fs';
// flowlint-next-line untyped-import:off
// @ts-expect-error - TS2307 - Cannot find module '@atlaspack/config-default' or its corresponding type declarations.
import defaultConfigContents from '@atlaspack/config-default';
import Module from 'module';
import path from 'path';
import {addHook} from 'pirates';
import Atlaspack, {INTERNAL_RESOLVE, INTERNAL_TRANSFORM} from '@atlaspack/core';

import syncPromise from './syncPromise';

let hooks: Record<string, any> = {};
// @ts-expect-error - TS7034 - Variable 'lastDisposable' implicitly has type 'any' in some locations where its type cannot be determined.
let lastDisposable;
let packageManager = new NodePackageManager(new NodeFS(), '/');
let defaultConfig = {
  ...defaultConfigContents,
  // @ts-expect-error - TS2339 - Property 'resolveSync' does not exist on type 'PackageManager'.
  filePath: packageManager.resolveSync('@atlaspack/config-default', __filename)
    .resolved,
};

function register(inputOpts?: InitialAtlaspackOptions): IDisposable {
  let opts: InitialAtlaspackOptions = {
    ...defaultConfig,
    ...(inputOpts || {}),
  };

  // Replace old hook, as this one likely contains options.
  // @ts-expect-error - TS7005 - Variable 'lastDisposable' implicitly has an 'any' type.
  if (lastDisposable) {
    // @ts-expect-error - TS7005 - Variable 'lastDisposable' implicitly has an 'any' type.
    lastDisposable.dispose();
  }

  let atlaspack = new Atlaspack({
    logLevel: 'error',
    ...opts,
  });

  let env = {
    context: 'node',
    engines: {
      node: process.versions.node,
    },
  };

  syncPromise(atlaspack._init());

  let isProcessing = false;

  // As Atlaspack is pretty much fully asynchronous, create an async function and wrap it in a syncPromise later...
  async function fileProcessor(code: string, filePath: string) {
    if (isProcessing) {
      return code;
    }

    try {
      isProcessing = true;
      // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'symbol' can't be used to index type 'Atlaspack'.
      let result = await atlaspack[INTERNAL_TRANSFORM]({
        filePath,
        env,
      });

      if (result.assets && result.assets.length >= 1) {
        let output = '';
        // @ts-expect-error - TS7006 - Parameter 'a' implicitly has an 'any' type.
        let asset = result.assets.find((a) => a.type === 'js');
        if (asset) {
          output = await asset.getCode();
        }
        return output;
      }
    } catch (e: any) {
      /* eslint-disable no-console */
      console.error('@atlaspack/register failed to process: ', filePath);
      console.error(e);
      /* eslint-enable */
    } finally {
      isProcessing = false;
    }

    return '';
  }

  // @ts-expect-error - TS7019 - Rest parameter 'args' implicitly has an 'any[]' type. | TS2556 - A spread argument must either have a tuple type or be passed to a rest parameter.
  let hookFunction = (...args) => syncPromise(fileProcessor(...args));

  function resolveFile(currFile: any, targetFile: any) {
    try {
      isProcessing = true;

      let resolved = syncPromise(
        // $FlowFixMe
        // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'symbol' can't be used to index type 'Atlaspack'.
        atlaspack[INTERNAL_RESOLVE]({
          specifier: targetFile,
          sourcePath: currFile,
          env,
        }),
      );

      // @ts-expect-error - TS2345 - Argument of type 'unknown' is not assignable to parameter of type 'string'.
      let targetFileExtension = path.extname(resolved);
      if (!hooks[targetFileExtension]) {
        hooks[targetFileExtension] = addHook(hookFunction, {
          exts: [targetFileExtension],
          ignoreNodeModules: false,
        });
      }

      return resolved;
    } finally {
      isProcessing = false;
    }
  }

  hooks.js = addHook(hookFunction, {
    exts: ['.js'],
    ignoreNodeModules: false,
  });

  // @ts-expect-error - TS7034 - Variable 'disposed' implicitly has type 'any' in some locations where its type cannot be determined.
  let disposed;

  // Patching Module._resolveFilename takes care of patching the underlying
  // resolver in both `require` and `require.resolve`:
  // https://github.com/nodejs/node-v0.x-archive/issues/1125#issuecomment-10748203
  // @ts-expect-error - TS2339 - Property '_resolveFilename' does not exist on type 'typeof Module'.
  const originalResolveFilename = Module._resolveFilename;
  // @ts-expect-error - TS2339 - Property '_resolveFilename' does not exist on type 'typeof Module'.
  Module._resolveFilename = function atlaspackResolveFilename(
    to: any,
    from: any,
    // @ts-expect-error - TS7019 - Rest parameter 'rest' implicitly has an 'any[]' type.
    ...rest
  ) {
    // @ts-expect-error - TS7005 - Variable 'disposed' implicitly has an 'any' type.
    return isProcessing || disposed
      ? originalResolveFilename(to, from, ...rest)
      : resolveFile(from?.filename, to);
  };

  let disposable = (lastDisposable = {
    dispose() {
      // @ts-expect-error - TS7005 - Variable 'disposed' implicitly has an 'any' type.
      if (disposed) {
        return;
      }

      for (let extension in hooks) {
        hooks[extension]();
      }

      disposed = true;
    },
  });

  return disposable;
}

let disposable: IDisposable = register();
register.dispose = (): unknown => disposable.dispose();

// Support both commonjs and ES6 modules
module.exports = register;
exports.default = register;
exports.__esModule = true;
