import type {
  Asset,
  BuildEvent,
  BuildSuccessEvent,
  BundleGraph,
  Dependency,
  FilePath,
  InitialAtlaspackOptions,
  PackagedBundle,
} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';
import {MemoryFS, ncp as _ncp, NodeFS, OverlayFS} from '@atlaspack/fs';
import type WorkerFarm from '@atlaspack/workers';
import type {IncomingMessage} from 'http';
import http from 'http';
import expect from 'expect';

import invariant from 'assert';
import assert from 'assert';
import util from 'util';
import Parcel, {createWorkerFarm} from '@atlaspack/core';
import vm from 'vm';
import v8 from 'v8';
import path from 'path';
import url from 'url';
import WebSocket from 'ws';
import nullthrows from 'nullthrows';
import {parser as postHtmlParse} from 'posthtml-parser';
import postHtml from 'posthtml';
import EventEmitter from 'events';
import https from 'https';

import {makeDeferredWithPromise, normalizeSeparators} from '@atlaspack/utils';
import _chalk from 'chalk';
import resolve from 'resolve';

export {fsFixture} from './fsFixture';

export const workerFarm = createWorkerFarm() as WorkerFarm;
export const inputFS: NodeFS = new NodeFS();
export let outputFS: MemoryFS = new MemoryFS(workerFarm);
export let overlayFS: OverlayFS = new OverlayFS(outputFS, inputFS);

beforeEach(() => {
  outputFS = new MemoryFS(workerFarm);
  overlayFS = new OverlayFS(outputFS, inputFS);
});

// Recursively copies a directory from the inputFS to the outputFS
export async function ncp(source: FilePath, destination: FilePath) {
  await _ncp(inputFS, source, outputFS, destination);
}

// Mocha is currently run with exit: true because of this issue preventing us
// from properly ending the workerfarm after the test run:
// https://github.com/nodejs/node/pull/28788
//
// TODO: Remove exit: true in .mocharc.json and instead add the following in this file:
//   // Spin down the worker farm to stop it from preventing the main process from exiting
//   await workerFarm.end();
// when https://github.com/nodejs/node/pull/28788 is resolved.

const chalk = new _chalk.Instance();
const warning = chalk.keyword('orange');

/* eslint-disable no-console */
console.warn = (...args) => {
  // eslint-disable-next-line no-console
  console.error(warning(...args));
};
/* eslint-enable no-console */

type ExternalModules = {
  [name: string]: (arg1: vm.Context) => {
    [key: string]: unknown;
  };
};

export function sleep(ms: number): Promise<void> {
  return new Promise(
    (resolve: (result: Promise<undefined> | undefined) => void) =>
      setTimeout(resolve, ms),
  );
}

export function normalizeFilePath(filePath: string): FilePath {
  return normalizeSeparators(filePath);
}

export const distDir: string = path.resolve(
  __dirname,
  '..',
  '..',
  'integration-tests',
  'dist',
);

export async function removeDistDirectory() {
  await outputFS.rimraf(distDir);
}

export function symlinkPrivilegeWarning() {
  // eslint-disable-next-line no-console
  console.warn(
    `-----------------------------------
Skipping symbolic link test(s) because you don't have the privilege.
Run tests with Administrator privilege.
If you don't know how, check here: https://bit.ly/2UmWsbD
-----------------------------------`,
  );
}

export const isAtlaspackV3 = process.env.ATLASPACK_V3 === 'true';

export function getParcelOptions(
  entries: FilePath | Array<FilePath>,
  opts?: Partial<InitialAtlaspackOptions>,
): InitialAtlaspackOptions {
  return mergeParcelOptions(
    {
      entries,
      shouldDisableCache: true,
      logLevel: 'none',
      shouldBundleIncrementally:
        process.env.NO_INCREMENTAL == null ? true : false,
      defaultConfig: path.join(__dirname, '.parcelrc-no-reporters'),
      inputFS,
      outputFS,
      workerFarm,
      shouldContentHash: true,
      defaultTargetOptions: {
        distDir,
        engines: {
          browsers: ['last 1 Chrome version'],
          node: '8',
        },
      },
      featureFlags: {
        atlaspackV3: isAtlaspackV3,
      },
    },
    opts,
  );
}

export function bundler(
  entries: FilePath | Array<FilePath>,
  opts?: Partial<InitialAtlaspackOptions>,
): Parcel {
  return new Parcel(getParcelOptions(entries, opts));
}

export function findAsset(
  bundleGraph: BundleGraph<PackagedBundle>,
  assetFileName: string,
): Asset | null | undefined {
  return bundleGraph.traverseBundles((bundle, context, actions) => {
    let asset = bundle.traverseAssets((asset, context, actions) => {
      if (path.basename(asset.filePath) === assetFileName) {
        actions.stop();
        return asset;
      }
    });
    if (asset) {
      actions.stop();
      return asset;
    }
  });
}

export function findDependency(
  bundleGraph: BundleGraph<PackagedBundle>,
  assetFileName: string,
  specifier: string,
): Dependency {
  let asset = nullthrows(
    findAsset(bundleGraph, assetFileName),
    `Couldn't find asset ${assetFileName}`,
  );

  let dependencies = bundleGraph
    .getDependencies(asset)
    .filter((d) => d.specifier === specifier);

  let dependency =
    dependencies.length > 1
      ? dependencies.find((d) => !bundleGraph.isDependencySkipped(d))
      : dependencies[0];

  invariant(
    dependency != null,
    `Couldn't find dependency ${assetFileName} -> ${specifier}`,
  );
  return dependency;
}

export function mergeParcelOptions(
  optsOne: InitialAtlaspackOptions,
  optsTwo?: InitialAtlaspackOptions | null,
): InitialAtlaspackOptions {
  if (!optsTwo) {
    return optsOne;
  }

  return {
    ...optsOne,
    ...optsTwo,
    // $FlowFixMe
    defaultTargetOptions: {
      ...optsOne?.defaultTargetOptions,
      // $FlowFixMe
      ...optsTwo?.defaultTargetOptions,
    },
    featureFlags: {
      // $FlowFixMe
      ...optsOne?.featureFlags,
      // $FlowFixMe
      ...optsTwo?.featureFlags,
    },
  };
}

export function assertDependencyWasExcluded(
  bundleGraph: BundleGraph<PackagedBundle>,
  assetFileName: string,
  specifier: string,
): void {
  let dep = findDependency(bundleGraph, assetFileName, specifier);
  invariant(
    bundleGraph.isDependencySkipped(dep),
    util.inspect(dep) + " wasn't deferred",
  );
}

export async function bundle(
  entries: FilePath | Array<FilePath>,
  opts?: InitialAtlaspackOptions,
): Promise<BundleGraph<PackagedBundle>> {
  return (await bundler(entries, opts).run()).bundleGraph;
}

export function getNextBuild(b: Parcel): Promise<BuildEvent> {
  return new Promise((resolve, reject: (error?: any) => void) => {
    let subscriptionPromise = b
      .watch((err, buildEvent) => {
        if (err) {
          reject(err);
          return;
        }

        subscriptionPromise
          .then((subscription) => {
            // If the watch callback was reached, subscription must have been successful
            invariant(subscription != null);
            return subscription.unsubscribe();
          })
          .then(() => {
            // If the build promise hasn't been rejected, buildEvent must exist
            invariant(buildEvent != null);
            resolve(buildEvent);
          })
          .catch(reject);
      })
      .catch(reject);
  });
}

export async function getNextBuildSuccess(
  b: Parcel,
): Promise<BuildSuccessEvent> {
  let evt = await getNextBuild(b);
  invariant(evt.type === 'buildSuccess');
  return evt;
}

export function shallowEqual(
  a: Partial<{
    readonly [key: string]: unknown;
  }>,
  b: Partial<{
    readonly [key: string]: unknown;
  }>,
): boolean {
  if (Object.keys(a).length !== Object.keys(b).length) {
    return false;
  }

  for (let [key, value] of Object.entries(a)) {
    if (!b.hasOwnProperty(key) || b[key] !== value) {
      return false;
    }
  }

  return true;
}

type RunOpts = {
  require?: boolean;
  strict?: boolean;
};

export async function runBundles(
  bundleGraph: BundleGraph<PackagedBundle>,
  parent: PackagedBundle,
  bundles: Array<[string, PackagedBundle]>,
  globals: unknown,
  opts: RunOpts = {},
  externalModules?: ExternalModules,
): Promise<unknown> {
  let entryAsset = nullthrows(
    bundles
      .map(([, b]: [any, any]) => b.getMainEntry() || b.getEntryAssets()[0])
      .filter(Boolean)[0],
  );
  let env = entryAsset.env;
  let target = env.context;
  let outputFormat = env.outputFormat;

  let ctx, promises;
  switch (target) {
    case 'browser': {
      let prepared = prepareBrowserContext(parent, globals);
      ctx = prepared.ctx;
      promises = prepared.promises;
      break;
    }
    case 'node':
    case 'electron-main':
      nodeCache.clear();
      ctx = prepareNodeContext(
        outputFormat === 'commonjs' && parent.filePath,
        globals,
        undefined,
        externalModules,
      );
      break;
    case 'electron-renderer': {
      nodeCache.clear();
      let prepared = prepareBrowserContext(parent, globals);
      prepareNodeContext(
        outputFormat === 'commonjs' && parent.filePath,
        globals,
        prepared.ctx,
        externalModules,
      );
      ctx = prepared.ctx;
      promises = prepared.promises;
      break;
    }
    case 'web-worker':
    case 'service-worker': {
      let prepared = prepareWorkerContext(parent.filePath, globals);
      ctx = prepared.ctx;
      promises = prepared.promises;
      break;
    }
    case 'worklet': {
      ctx = Object.assign({}, globals);
      break;
    }
    default:
      throw new Error('Unknown target ' + target);
  }

  // A utility to prevent optimizers from removing side-effect-free code needed for testing
  ctx.sideEffectNoop = (v: any) => v;

  vm.createContext(ctx);
  let esmOutput;
  if (outputFormat === 'esmodule') {
    let res = await runESM(
      bundles[0][1].target.distDir,
      bundles.map(([code, bundle]: [any, any]) => [code, bundle.filePath]),
      ctx,
      overlayFS,
      externalModules,
      true,
    );

    esmOutput = bundles.length === 1 ? res[0] : res;
  } else {
    for (let [code, b] of bundles) {
      // require, parcelRequire was set up in prepare*Context
      new vm.Script((opts.strict ? '"use strict";\n' : '') + code, {
        filename:
          b.bundleBehavior === 'inline'
            ? b.name
            : normalizeSeparators(path.relative(b.target.distDir, b.filePath)),
        async importModuleDynamically(specifier: any) {
          let filePath = path.resolve(path.dirname(parent.filePath), specifier);
          let code = await overlayFS.readFile(filePath, 'utf8');
          let modules = await runESM(
            b.target.distDir,
            [[code, filePath]],
            ctx,
            overlayFS,
            externalModules,
            true,
          );
          return modules[0];
        },
      }).runInContext(ctx);
    }
  }
  if (promises) {
    // await any ongoing dynamic imports during the run
    await Promise.all(promises);
  }

  if (opts.require !== false) {
    switch (outputFormat) {
      case 'global':
        if (env.shouldScopeHoist) {
          return typeof ctx.output !== 'undefined' ? ctx.output : undefined;
        } else {
          for (let key in ctx) {
            if (key.startsWith('parcelRequire')) {
              return ctx[key](bundleGraph.getAssetPublicId(entryAsset));
            }
          }
        }
        return;
      case 'commonjs':
        invariant(typeof ctx.module === 'object' && ctx.module != null);
        return ctx.module.exports;
      case 'esmodule':
        return esmOutput;
      default:
        throw new Error(
          'Unable to run bundle with outputFormat ' + env.outputFormat,
        );
    }
  }

  return ctx;
}

export async function runBundle(
  bundleGraph: BundleGraph<PackagedBundle>,
  bundle: PackagedBundle,
  globals: unknown,
  opts: RunOpts = {},
  externalModules?: ExternalModules,
): Promise<unknown> {
  if (bundle.type === 'html') {
    let code = await overlayFS.readFile(nullthrows(bundle.filePath), 'utf8');
    let ast = postHtmlParse(code, {
      lowerCaseAttributeNames: true,
    });

    let bundles = bundleGraph.getBundles({includeInline: true});
    let scripts: Array<[string, PackagedBundle]> = [];
    postHtml().walk.call(ast, (node) => {
      if (node.attrs?.nomodule != null) {
        return node;
      }
      if (node.tag === 'script' && node.attrs?.src) {
        let src = url.parse(nullthrows(node.attrs).src);
        if (src.hostname == null) {
          let p = path.join(distDir, nullthrows(src.pathname));
          let b = nullthrows(bundles.find((b) => b.filePath === p));
          scripts.push([overlayFS.readFileSync(b.filePath, 'utf8'), b]);
        }
      } else if (node.tag === 'script' && node.content && !node.attrs?.src) {
        let content = node.content.join('');
        let inline = bundles.filter(
          (b) => b.bundleBehavior === 'inline' && b.type === 'js',
        );
        scripts.push([content, inline[0]]);
      }
      return node;
    });

    return runBundles(
      bundleGraph,
      bundle,
      scripts,
      globals,
      opts,
      externalModules,
    );
  } else {
    return runBundles(
      bundleGraph,
      bundle,
      [[overlayFS.readFileSync(bundle.filePath, 'utf8'), bundle]],
      globals,
      opts,
      externalModules,
    );
  }
}

export function run(
  bundleGraph: BundleGraph<PackagedBundle>,
  globals: unknown,
  opts: RunOpts = {},
  // $FlowFixMe[unclear-type]
  externalModules?: ExternalModules,
): Promise<any> {
  let bundle = nullthrows(
    bundleGraph.getBundles().find((b) => b.type === 'js' || b.type === 'html'),
  );
  return runBundle(bundleGraph, bundle, globals, opts, externalModules);
}

export function getBundleData(
  bundleGraph: BundleGraph<PackagedBundle>,
  inputDir: string,
): {
  name: string;
  type: string;
  assets: string[];
}[] {
  const byAlphabet = (a: string, b: string) =>
    a.toLowerCase() < b.toLowerCase() ? -1 : 1;
  const bundles = bundleGraph.getBundles();
  const bundleData = bundles.map((bundle) => {
    const assets: Array<string> = [];
    bundle.traverseAssets((asset) => {
      assets.push(path.relative(inputDir, asset.filePath));
    });
    assets.sort(byAlphabet);
    return {name: bundle.name, type: bundle.type, assets};
  });
  bundleData.sort(({name: a}, {name: b}) => byAlphabet(a, b));
  return bundleData;
}

export function expectBundles(
  inputDir: string,
  bundleGraph: BundleGraph<PackagedBundle>,
  expectedBundles: Array<{
    name?: string | RegExp;
    type?: string;
    assets: Array<string>;
  }>,
) {
  const bundleData = getBundleData(bundleGraph, inputDir);
  expect(bundleData).toEqual(expectedBundles);
}

export function assertBundles(
  bundleGraph: BundleGraph<PackagedBundle>,
  expectedBundles: Array<{
    name?: string | RegExp;
    type?: string;
    assets: Array<string>;
  }>,
) {
  let actualBundles: Array<
    | any
    | {
        assets: Array<string>;
        name: string;
        type: string;
      }
  > = [];
  const byAlphabet = (a: string, b: string) =>
    a.toLowerCase() < b.toLowerCase() ? -1 : 1;

  bundleGraph.traverseBundles((bundle) => {
    let assets: Array<string> = [];

    bundle.traverseAssets((asset) => {
      if (/@swc[/\\]helpers/.test(asset.filePath)) {
        // Skip all helpers for now, as they add friction and churn to assertions.
        // A longer term solution might have an explicit opt-in to this behavior, or
        // if we enable symbol propagation unconditionally, the set of helpers
        // should be more minimal.
        return;
      }

      if (/runtime-[a-z0-9]{16}\.js/.test(asset.filePath)) {
        // Skip runtime assets, which have hashed filenames for source maps.
        return;
      }

      const name = path.basename(asset.filePath);
      assets.push(name);
    });

    assets.sort(byAlphabet);
    actualBundles.push({
      name:
        bundle.bundleBehavior === 'inline'
          ? bundle.name
          : path.basename(bundle.filePath),
      type: bundle.type,
      assets,
    });
  });

  for (let bundle of expectedBundles) {
    if (!Array.isArray(bundle.assets)) {
      throw new Error(
        'Expected bundle must include an array of expected assets',
      );
    }
    bundle.assets.sort(byAlphabet);
  }

  assert.equal(
    actualBundles.length,
    expectedBundles.length,
    'expected number of bundles mismatched',
  );

  for (let bundle of expectedBundles) {
    let name = bundle.name;
    let found = actualBundles.some((b) => {
      if (name != null && b.name != null) {
        if (typeof name === 'string') {
          if (name !== b.name) {
            return false;
          }
        } else if (name instanceof RegExp) {
          if (!name.test(b.name)) {
            return false;
          }
        } else {
          assert.fail('Expected bundle name has invalid type');
        }
      }

      if (bundle.type != null && bundle.type !== b.type) {
        return false;
      }

      return (
        bundle.assets &&
        bundle.assets.length === b.assets.length &&
        bundle.assets.every((a, i) => a === b.assets[i])
      );
    });

    if (!found) {
      assert.fail(
        `Could not find expected bundle: \n\n${util.inspect(
          bundle,
        )} \n\nActual bundles: \n\n${util.inspect(actualBundles)}`,
      );
    }
  }
}

export function normaliseNewlines(text: string): string {
  return text.replace(/(\r\n|\n|\r)/g, '\n');
}

function prepareBrowserContext(
  bundle: PackagedBundle,
  globals: unknown,
): {
  ctx: vm.Context;
  promises: Array<Promise<unknown>>;
} {
  // for testing dynamic imports
  const fakeElement = {
    remove() {},
  } as const;

  const head = {
    children: [],
    appendChild(el: any) {
      head.children.push(el);

      if (el.tag === 'script') {
        let {deferred, promise} = makeDeferredWithPromise();
        promises.push(promise);
        setTimeout(function () {
          let pathname = url.parse(el.src).pathname;
          let file = path.join(bundle.target.distDir, pathname);

          new vm.Script(
            // '"use strict";\n' +
            overlayFS.readFileSync(file, 'utf8'),
            {
              filename: pathname.slice(1),
            },
          ).runInContext(ctx);

          el.onload();
          deferred.resolve();
        }, 0);
      } else if (typeof el.onload === 'function') {
        el.onload();
      }
    },
  } as const;

  let promises: Array<Promise<unknown>> = [];

  const fakeDocument = {
    head,
    createElement(tag: any) {
      return {tag};
    },

    getElementsByTagName() {
      return [head];
    },

    createEvent() {
      return {timeStamp: Date.now()};
    },

    getElementById(id: any) {
      if (id !== '__parcel__error__overlay__') return fakeElement;
    },

    body: {
      appendChild() {
        return null;
      },
    },

    currentScript: null,
  } as const;

  var exports: Record<string, any> = {};

  function PatchedError(message: any) {
    const patchedError = new Error(message);
    const stackStart = patchedError.stack.match(/at (new )?Error/)?.index;
    const stackEnd = patchedError.stack.includes('at Script.runInContext')
      ? patchedError.stack.indexOf('at Script.runInContext')
      : patchedError.stack.indexOf('at runNextTicks');
    const stack = patchedError.stack.slice(stackStart, stackEnd).split('\n');
    stack.shift();
    stack.pop();
    for (let [i, line] of stack.entries()) {
      stack[i] = line.replace(
        /( ?.* )\(?(.*)\)?$/,
        (_, prefix, path) =>
          prefix +
          (path.endsWith(')')
            ? `(http://localhost/${path.slice(0, path.length - 1)})`
            : `http://localhost/${path}`),
      );
    }
    patchedError.stack =
      patchedError.stack.slice(0, stackStart).replace(/ +$/, '') +
      stack.join('\n');

    return patchedError;
  }

  PatchedError.prototype = Object.create(Error.prototype);
  Object.defineProperty(PatchedError, 'name', {
    writable: true,
    value: 'Error',
  });
  PatchedError.prototype.constructor = PatchedError;

  var ctx = Object.assign(
    {
      Error: PatchedError,
      exports,
      module: {exports},
      document: fakeDocument,
      WebSocket,
      TextEncoder,
      TextDecoder,
      console: {
        ...console,
        clear: () => {},
      },
      location: {
        hostname: 'localhost',
        origin: 'http://localhost',
        protocol: 'http',
      },
      navigator: {
        userAgent: '',
      },
      fetch(url: any) {
        return Promise.resolve({
          async arrayBuffer() {
            let readFilePromise = overlayFS.readFile(
              path.join(path.dirname(bundle.target.distDir), url),
            );
            promises.push(readFilePromise);
            return new Uint8Array(await readFilePromise).buffer;
          },
          text() {
            let readFilePromise = overlayFS.readFile(
              path.join(path.dirname(bundle.target.distDir), url),
              'utf8',
            );
            promises.push(readFilePromise);
            return readFilePromise;
          },
        });
      },
      atob(str: any) {
        return Buffer.from(str, 'base64').toString('binary');
      },
      btoa(str: any) {
        return Buffer.from(str, 'binary').toString('base64');
      },
      URL,
      Worker: createWorkerClass(bundle.filePath),
      addEventListener() {},
      removeEventListener() {},
    },
    globals,
  );

  ctx.window = ctx.self = ctx;
  return {ctx, promises};
}

function createWorkerClass(filePath: FilePath) {
  return class Worker extends EventEmitter {
    constructor(url: any) {
      super();
      this._run(url);
    }

    async _run(url: any) {
      let u = new URL(url);
      let filename = path.join(path.dirname(filePath), u.pathname);
      let {ctx, promises} = prepareWorkerContext(filename, {
        postMessage: (msg) => {
          this.emit('message', msg);
        },
      });

      let code = await overlayFS.readFile(filename, 'utf8');
      vm.createContext(ctx);
      new vm.Script(code, {
        filename: 'http://localhost/' + path.basename(filename),
      }).runInContext(ctx);

      if (promises) {
        await Promise.all(promises);
      }
    }

    addEventListener(evt: any, callback: any) {
      super.on(evt, callback);
    }

    removeEventListener(evt: any, callback: any) {
      super.removeListener(evt, callback);
    }
  };
}

function prepareWorkerContext(
  filePath: FilePath,
  globals: unknown,
): {
  ctx: vm.Context;
  promises: Array<Promise<unknown>>;
} {
  let promises: Array<Promise<unknown>> = [];

  var exports: Record<string, any> = {};
  var ctx = Object.assign(
    {
      exports,
      module: {exports},
      WebSocket,
      console,
      TextEncoder,
      TextDecoder,
      location: {hostname: 'localhost', origin: 'http://localhost'},
      importScripts(...urls) {
        for (let u of urls) {
          new vm.Script(
            overlayFS.readFileSync(
              path.join(path.dirname(filePath), url.parse(u).pathname),
              'utf8',
            ),
            {
              filename: path.basename(url.parse(u).pathname),
            },
          ).runInContext(ctx);
        }
      },
      fetch(url: any) {
        return Promise.resolve({
          async arrayBuffer() {
            let readFilePromise = overlayFS.readFile(
              path.join(path.dirname(filePath), url),
            );
            promises.push(readFilePromise);
            return new Uint8Array(await readFilePromise).buffer;
          },
          text() {
            let readFilePromise = overlayFS.readFile(
              path.join(path.dirname(filePath), url),
              'utf8',
            );
            promises.push(readFilePromise);
            return readFilePromise;
          },
        });
      },
      atob(str: any) {
        return Buffer.from(str, 'base64').toString('binary');
      },
      btoa(str: any) {
        return Buffer.from(str, 'binary').toString('base64');
      },
      URL,
      Worker: createWorkerClass(filePath),
    },
    globals,
  );

  ctx.window = ctx.self = ctx;
  return {ctx, promises};
}

const nodeCache = new Map();

// no filepath = ESM
function prepareNodeContext(
  filePath,
  globals: unknown,
  // $FlowFixMe
  ctx: any = {},
  externalModules?: ExternalModules,
) {
  let exports: Record<string, any> = {};
  let req =
    filePath &&
    ((specifier: any) => {
      if (externalModules && specifier in externalModules) {
        return externalModules[specifier](ctx);
      }

      let res = resolve.sync(specifier, {
        basedir: path.dirname(filePath),
        preserveSymlinks: true,
        extensions: ['.js', '.json'],
        readFileSync: (...args) => {
          return overlayFS.readFileSync(...args);
        },
        isFile: (file) => {
          try {
            var stat = overlayFS.statSync(file);
          } catch (err: any) {
            return false;
          }
          return stat.isFile();
        },
        isDirectory: (file) => {
          try {
            var stat = overlayFS.statSync(file);
          } catch (err: any) {
            return false;
          }
          return stat.isDirectory();
        },
      });

      // Shim FS module using overlayFS
      if (res === 'fs') {
        return {
          readFile: async (file, encoding, cb) => {
            let res = await overlayFS.readFile(file, encoding);
            cb(null, res);
          },
          readFileSync: (file, encoding) => {
            return overlayFS.readFileSync(file, encoding);
          },
        };
      }

      if (res === specifier) {
        return require(specifier);
      }

      if (path.extname(res) === '.css') {
        return {};
      }

      let cached = nodeCache.get(res);
      if (cached) {
        return cached.module.exports;
      }

      let g = {
        ...globals,
      };

      for (let key in ctx) {
        if (
          key !== 'module' &&
          key !== 'exports' &&
          key !== '__filename' &&
          key !== '__dirname' &&
          key !== 'require'
        ) {
          g[key] = ctx[key];
        }
      }

      let childCtx = prepareNodeContext(res, g);
      nodeCache.set(res, childCtx);

      vm.createContext(childCtx);
      new vm.Script(
        //'"use strict";\n' +
        overlayFS.readFileSync(res, 'utf8'),
        {
          filename: path.basename(res),
        },
      ).runInContext(childCtx);
      return childCtx.module.exports;
    });

  if (filePath) {
    ctx.module = {exports, require: req};
    ctx.exports = exports;
    ctx.__filename = filePath;
    ctx.__dirname = path.dirname(filePath);
    ctx.require = req;
  }

  ctx.console = console;
  ctx.process = process;
  ctx.setTimeout = setTimeout;
  ctx.setImmediate = setImmediate;
  ctx.global = ctx;
  ctx.URL = URL;
  ctx.TextEncoder = TextEncoder;
  ctx.TextDecoder = TextDecoder;
  Object.assign(ctx, globals);
  return ctx;
}

let instanceId = 0;

export async function runESM(
  baseDir: FilePath,
  entries: Array<[string, string]>,
  context: vm.Context,
  fs: FileSystem,
  externalModules: ExternalModules = {},
  requireExtensions: boolean = false,
): Promise<
  Array<{
    [key: string]: unknown;
  }>
> {
  let id = instanceId++;
  let cache = new Map();

  function load(inputSpecifier, referrer, code = null) {
    // ESM can request bundles with an absolute URL. Normalize this to the baseDir.
    let specifier = inputSpecifier.replace('http://localhost', baseDir);

    if (path.isAbsolute(specifier) || specifier.startsWith('.')) {
      let extname = path.extname(specifier);
      if (
        extname &&
        extname !== '.js' &&
        extname !== '.mjs' &&
        extname !== '.css'
      ) {
        throw new Error(
          'Unknown file extension in ' +
            specifier +
            ' from ' +
            referrer.identifier,
        );
      }
      let filename = path.resolve(
        baseDir,
        path.dirname(referrer.identifier),
        !extname && !requireExtensions ? specifier + '.js' : specifier,
      );

      let m = cache.get(filename);
      if (m) {
        return m;
      }

      let source =
        code ??
        (extname === '.css' ? '' : null) ??
        fs.readFileSync(filename, 'utf8');
      m = new vm.SourceTextModule(source, {
        identifier: `${normalizeSeparators(
          path.relative(baseDir, filename),
        )}?id=${id}`,
        importModuleDynamically: (specifier, referrer) =>
          entry(specifier, referrer),
        context,
        initializeImportMeta(meta: any) {
          meta.url = `http://localhost/${path.basename(filename)}`;
        },
      });
      cache.set(filename, m);
      return m;
    } else {
      if (!(specifier in externalModules)) {
        throw new Error(
          `Couldn't resolve ${specifier} from ${referrer.identifier}`,
        );
      }

      let m = cache.get(specifier);
      if (m) {
        return m;
      }

      let ns = externalModules[specifier](context);

      m = new vm.SyntheticModule(
        Object.keys(ns),
        function () {
          for (let [k, v] of Object.entries(ns)) {
            this.setExport(k, v);
          }
        },
        {identifier: specifier, context},
      );
      cache.set(specifier, m);
      return m;
    }
  }

  async function _entry(m: any) {
    if (m.status === 'unlinked') {
      await m.link((specifier, referrer) => load(specifier, referrer));
    }
    if (m.status === 'linked') {
      await m.evaluate();
    }
    return m;
  }

  let entryPromises = new Map();

  function entry(
    specifier: string,
    referrer: {
      identifier: string;
    },
    code: undefined | string,
  ) {
    let m = load(specifier, referrer, code);
    let promise = entryPromises.get(m);
    if (!promise) {
      promise = _entry(m);
      entryPromises.set(m, promise);
    }
    return promise;
  }

  let modules: Array<never> = [];
  for (let [code, f] of entries) {
    modules.push(await entry(f, {identifier: ''}, code));
  }

  for (let m of modules) {
    if (m.status === 'errored') {
      throw m.error;
    }
  }

  return modules.map((m) => m.namespace);
}

export async function assertESMExports(
  b: BundleGraph<PackagedBundle>,
  expected: unknown,
  externalModules?: ExternalModules,
  // $FlowFixMe[unclear-type]
  evaluate?: ((arg1: {[key: string]: any}) => unknown) | null,
) {
  let parcelResult = await run(b, undefined, undefined, externalModules);

  let entry = nullthrows(
    b
      .getBundles()
      .find((b) => b.type === 'js')
      ?.getMainEntry(),
  );
  nodeCache.clear();
  let [nodeResult] = await runESM(
    b.getBundles()[0].target.distDir,
    [[await inputFS.readFile(entry.filePath, 'utf8'), entry.filePath]],
    vm.createContext(prepareNodeContext(false, {})),
    inputFS,
    externalModules,
  );

  if (evaluate) {
    parcelResult = await evaluate(parcelResult);
    nodeResult = await evaluate(nodeResult);
  }
  assert.deepEqual(
    parcelResult,
    nodeResult,
    "Bundle exports don't match Node's native behaviour",
  );

  if (!evaluate) {
    parcelResult = {...parcelResult};
  }
  assert.deepEqual(parcelResult, expected);
}

export async function assertNoFilePathInCache(
  fs: FileSystem,
  dir: string,
  projectRoot: string,
) {
  let entries = await fs.readdir(dir);
  for (let entry of entries) {
    // Skip watcher snapshots for linux/windows, which contain full file paths.
    if (path.extname(entry) === '.txt') {
      continue;
    }

    let fullPath = path.join(dir, entry);
    let stat = await fs.stat(fullPath);
    if (stat.isDirectory()) {
      await assertNoFilePathInCache(fs, fullPath, projectRoot);
    } else if (stat.isFile()) {
      let contents = await fs.readFile(fullPath);

      // For debugging purposes, log all instances of the projectRoot in the cache.
      // Otherwise, fail the test if one is found.
      if (process.env.ATLASPACK_DEBUG_CACHE_FILEPATH != null) {
        if (contents.includes(projectRoot)) {
          let deserialized;
          try {
            deserialized = v8.deserialize(contents);
          } catch (err: any) {
            // rudimentary detection of binary files
            if (!contents.includes(0)) {
              deserialized = contents.toString();
            } else {
              deserialized = contents;
            }
          }

          if (deserialized != null) {
            // eslint-disable-next-line no-console
            console.log(
              `Found projectRoot ${projectRoot} in cache file ${fullPath}`,
            );
            // eslint-disable-next-line no-console
            console.log(
              require('util').inspect(deserialized, {depth: 50, colors: true}),
            );
          }
        }
      } else {
        assert(
          !contents.includes(projectRoot),
          `Found projectRoot ${projectRoot} in cache file ${fullPath}`,
        );
      }
    }
  }
}

export function requestRaw(
  file: string,
  port: number,
  options?: requestOptions | null,
  client: typeof http | typeof https = http,
): Promise<{
  res: IncomingMessage;
  data: string;
}> {
  return new Promise((resolve, reject: (error?: any) => void) => {
    client
      // $FlowFixMe
      .request(
        {
          hostname: 'localhost',
          port: port,
          path: file,
          rejectUnauthorized: false,
          ...options,
        },
        (res: IncomingMessage) => {
          res.setEncoding('utf8');
          let data = '';
          res.on('data', (c) => (data += c));
          res.on('end', () => {
            if (res.statusCode !== 200) {
              return reject({res, data});
            }

            resolve({res, data});
          });
        },
      )
      .end();
  });
}

export function request(
  file: string,
  port: number,
  client: typeof http | typeof https = http,
): Promise<string> {
  return new Promise(
    (
      resolve: (result: Promise<string> | string) => void,
      reject: (error?: any) => void,
    ) => {
      client.get(
        {
          hostname: 'localhost',
          port: port,
          path: file,
          rejectUnauthorized: false,
        },
        (res) => {
          res.setEncoding('utf8');
          let data = '';
          res.on('data', (c) => (data += c));
          res.on('end', () => {
            if (res.statusCode !== 200) {
              return reject({statusCode: res.statusCode, data});
            }

            resolve(data);
          });
        },
      );
    },
  );
}

let origDescribe = globalThis.describe;
let parcelVersion: string | undefined;

export function describe(...args: unknown[]) {
  parcelVersion = undefined;
  origDescribe.apply(this, args);
}

describe.only = function (...args: unknown[]) {
  parcelVersion = undefined;
  origDescribe.only.apply(this, args);
};

describe.skip = function (...args: unknown[]) {
  parcelVersion = undefined;
  origDescribe.skip.apply(this, args);
};

describe.v2 = function (...args: unknown[]) {
  parcelVersion = 'v2';
  if (!isAtlaspackV3) {
    origDescribe.apply(this, args);
  }
};

describe.v2.only = function (...args: unknown[]) {
  parcelVersion = 'v2';
  if (!isAtlaspackV3) {
    origDescribe.only.apply(this, args);
  }
};

describe.v3 = function (...args: unknown[]) {
  parcelVersion = 'v3';
  if (isAtlaspackV3) {
    origDescribe.apply(this, args);
  }
};

describe.v3.only = function (...args: unknown[]) {
  parcelVersion = 'v3';
  if (isAtlaspackV3) {
    origDescribe.only.apply(this, args);
  }
};

let origIt = globalThis.it;

export function it(...args: unknown[]) {
  if (
    parcelVersion == null ||
    (parcelVersion == 'v2' && !isAtlaspackV3) ||
    (parcelVersion == 'v3' && isAtlaspackV3)
  ) {
    origIt.apply(this, args);
  }
}

it.only = function (...args: unknown[]) {
  origIt.only.apply(this, args);
};

it.skip = function (...args: unknown[]) {
  origIt.skip.apply(this, args);
};

it.v2 = function (...args: unknown[]) {
  if (!isAtlaspackV3) {
    origIt.apply(this, args);
  }
};

it.v2.only = function (...args: unknown[]) {
  if (!isAtlaspackV3) {
    origIt.only.apply(this, args);
  }
};

it.v3 = function (...args: unknown[]) {
  if (isAtlaspackV3) {
    origIt.apply(this, args);
  }
};

it.v3.only = function (...args: unknown[]) {
  if (isAtlaspackV3) {
    origIt.only.apply(this, args);
  }
};