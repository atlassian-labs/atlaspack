/* global HMR_HOST, HMR_PORT, HMR_ENV_HASH, HMR_SECURE, HMR_USE_SSE, chrome, browser, __parcel__import__, __parcel__importScripts__, ServiceWorkerGlobalScope */

/*::
import type {
  HMRAsset,
  HMRMessage,
} from '@atlaspack/reporter-dev-server/src/HMRServer';
interface ParcelRequire {
  (string): mixed;
  cache: {|[string]: ParcelModule|};
  hotData: {|[string]: mixed|};
  Module: any;
  parent: ?ParcelRequire;
  isParcelRequire: true;
  modules: {|[string]: [Function, {|[string]: string|}]|};
  HMR_BUNDLE_ID: string;
  root: ParcelRequire;
}
interface ParcelModule {
  hot: {|
    data: mixed,
    accept(cb: (Function) => void): void,
    dispose(cb: (mixed) => void): void,
    // accept(deps: Array<string> | string, cb: (Function) => void): void,
    // decline(): void,
    _acceptCallbacks: Array<(Function) => void>,
    _disposeCallbacks: Array<(mixed) => void>,
  |};
}
interface ExtensionContext {
  runtime: {|
    reload(): void,
    getURL(url: string): string;
    getManifest(): {manifest_version: number, ...};
  |};
}
declare var module: {bundle: ParcelRequire, ...};
declare var HMR_HOST: string;
declare var HMR_PORT: string;
declare var HMR_ENV_HASH: string;
declare var HMR_SECURE: boolean;
declare var HMR_USE_SSE: boolean;
declare var chrome: ExtensionContext;
declare var browser: ExtensionContext;
declare var __parcel__import__: (string) => Promise<void>;
declare var __parcel__importScripts__: (string) => Promise<void>;
declare var globalThis: typeof self;
declare var ServiceWorkerGlobalScope: Object;
*/

// flow-to-ts helpers
export type SetComplement<A, B extends A> = A extends B ? never : A;
export type Diff<T extends U, U extends object> = Pick<
  T,
  SetComplement<keyof T, keyof U>
>;
// /flow-to-ts helpers

var OVERLAY_ID = '__parcel__error__overlay__';

// @ts-expect-error TS2339
var OldModule = module.bundle.Module;

function Module(moduleName: any) {
  // @ts-expect-error TS2683
  OldModule.call(this, moduleName);
  // @ts-expect-error TS2683
  this.hot = {
    // @ts-expect-error TS2339
    data: module.bundle.hotData[moduleName],
    _acceptCallbacks: [],
    _disposeCallbacks: [],
    // @ts-expect-error TS7006
    accept: function (fn) {
      this._acceptCallbacks.push(fn || function () {});
    },
    // @ts-expect-error TS7006
    dispose: function (fn) {
      this._disposeCallbacks.push(fn);
    },
  };
  // @ts-expect-error TS2339
  module.bundle.hotData[moduleName] = undefined;
}
// @ts-expect-error TS2339
module.bundle.Module = Module;
// @ts-expect-error TS2339
module.bundle.hotData = {};

// @ts-expect-error TS7034
var checkedAssets /*: {|[string]: boolean|} */,
  // @ts-expect-error TS7034
  disposedAssets /*: {|[string]: boolean|} */,
  // @ts-expect-error TS7034
  assetsToDispose /*: Array<[ParcelRequire, string]> */,
  // @ts-expect-error TS7034
  assetsToAccept /*: Array<[ParcelRequire, string]> */;

function getHostname() {
  return (
    // @ts-expect-error TS2304
    HMR_HOST ||
    (location.protocol.indexOf('http') === 0 ? location.hostname : 'localhost')
  );
}

function getPort() {
  // @ts-expect-error TS2304
  return HMR_PORT || location.port;
}

// eslint-disable-next-line no-redeclare
// @ts-expect-error TS2339
var parent = module.bundle.parent;
if ((!parent || !parent.isParcelRequire) && typeof WebSocket !== 'undefined') {
  var hostname = getHostname();
  var port = getPort();
  var protocol =
    // @ts-expect-error TS2304
    HMR_SECURE ||
    (location.protocol == 'https:' &&
      !['localhost', '127.0.0.1', '0.0.0.0'].includes(hostname))
      ? 'wss'
      : 'ws';

  var ws;
  // @ts-expect-error TS2304
  if (HMR_USE_SSE) {
    ws = new EventSource('/__parcel_hmr');
  } else {
    try {
      ws = new WebSocket(
        protocol + '://' + hostname + (port ? ':' + port : '') + '/',
      );
    } catch (err: any) {
      if (err.message) {
        console.error(err.message);
      }
      ws = {};
    }
  }

  // Web extension context
  var extCtx =
    // @ts-expect-error TS2304
    typeof browser === 'undefined'
      ? // @ts-expect-error TS2304
        typeof chrome === 'undefined'
        ? null
        : // @ts-expect-error TS2304
          chrome
      : // @ts-expect-error TS2304
        browser;

  // Safari doesn't support sourceURL in error stacks.
  // eval may also be disabled via CSP, so do a quick check.
  var supportsSourceURL = false;
  try {
    (0, eval)('throw new Error("test"); //# sourceURL=test.js');
  } catch (err: any) {
    supportsSourceURL = err.stack.includes('test.js');
  }

  // @ts-expect-error TS2339
  ws.onmessage = async function (
    event: {
      data: string;
    } /*: {data: string, ...} */,
  ) {
    checkedAssets = {} /*: {|[string]: boolean|} */;
    disposedAssets = {} /*: {|[string]: boolean|} */;
    assetsToAccept = [];
    assetsToDispose = [];

    var data /*: HMRMessage */ = JSON.parse(event.data);

    if (data.type === 'reload') {
      fullReload();
    } else if (data.type === 'update') {
      // Remove error overlay if there is one
      if (typeof document !== 'undefined') {
        removeErrorOverlay();
      }

      let assets = data.assets.filter(
        // @ts-expect-error TS7006
        (asset) => asset.envHash === HMR_ENV_HASH,
      );

      // Handle HMR Update
      // @ts-expect-error TS7006
      let handled = assets.every((asset) => {
        return (
          asset.type === 'css' ||
          (asset.type === 'js' &&
            // @ts-expect-error TS2339
            hmrAcceptCheck(module.bundle.root, asset.id, asset.depsByBundle))
        );
      });

      if (handled) {
        console.clear();

        // Dispatch custom event so other runtimes (e.g React Refresh) are aware.
        if (
          typeof window !== 'undefined' &&
          typeof CustomEvent !== 'undefined'
        ) {
          window.dispatchEvent(new CustomEvent('parcelhmraccept'));
        }

        await hmrApplyUpdates(assets);

        hmrDisposeQueue();

        // Run accept callbacks. This will also re-execute other disposed assets in topological order.
        let processedAssets: Record<string, any> = {};
        for (let i = 0; i < assetsToAccept.length; i++) {
          // @ts-expect-error TS7005
          let id = assetsToAccept[i][1];

          if (!processedAssets[id]) {
            // @ts-expect-error TS7005
            hmrAccept(assetsToAccept[i][0], id);
            processedAssets[id] = true;
          }
        }
      } else fullReload();
    }

    if (data.type === 'error') {
      // Log parcel errors to console
      for (let ansiDiagnostic of data.diagnostics.ansi) {
        let stack = ansiDiagnostic.codeframe
          ? ansiDiagnostic.codeframe
          : ansiDiagnostic.stack;

        console.error(
          '🚨 [parcel]: ' +
            ansiDiagnostic.message +
            '\n' +
            stack +
            '\n\n' +
            ansiDiagnostic.hints.join('\n'),
        );
      }

      if (typeof document !== 'undefined') {
        // Render the fancy html overlay
        removeErrorOverlay();
        var overlay = createErrorOverlay(data.diagnostics.html);
        document.body.appendChild(overlay);
      }
    }
  };
  if (ws instanceof WebSocket) {
    ws.onerror = function (e: any) {
      if (e.message) {
        console.error(e.message);
      }
    };
    ws.onclose = function (e: CloseEvent) {
      if (process.env.ATLASPACK_BUILD_ENV !== 'test') {
        console.warn('[parcel] 🚨 Connection to the HMR server was lost');
      }
    };
  }
}

function removeErrorOverlay() {
  var overlay = document.getElementById(OVERLAY_ID);
  if (overlay) {
    overlay.remove();
    console.log('[parcel] ✨ Error resolved');
  }
}

function createErrorOverlay(
  diagnostics: Array<
    Partial<
      Diff<
        // @ts-expect-error TS2304
        AnsiDiagnosticResult,
        {
          codeframe: string;
        }
      >
    >
  >,
) {
  var overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;

  let errorHTML =
    '<div style="background: black; opacity: 0.85; font-size: 16px; color: white; position: fixed; height: 100%; width: 100%; top: 0px; left: 0px; padding: 30px; font-family: Menlo, Consolas, monospace; z-index: 9999;">';

  for (let diagnostic of diagnostics) {
    let stack = diagnostic.frames.length
      ? // @ts-expect-error TS7006
        diagnostic.frames.reduce((p, frame) => {
          return `${p}
<a href="/__parcel_launch_editor?file=${encodeURIComponent(
            frame.location,
          )}" style="text-decoration: underline; color: #888" onclick="fetch(this.href); return false">${
            frame.location
          }</a>
${frame.code}`;
        }, '')
      : diagnostic.stack;

    errorHTML += `
      <div>
        <div style="font-size: 18px; font-weight: bold; margin-top: 20px;">
          🚨 ${diagnostic.message}
        </div>
        <pre>${stack}</pre>
        <div>
          ${diagnostic.hints
            // @ts-expect-error TS7006
            .map((hint) => '<div>💡 ' + hint + '</div>')
            .join('')}
        </div>
        ${
          diagnostic.documentation
            ? `<div>📝 <a style="color: violet" href="${diagnostic.documentation}" target="_blank">Learn more</a></div>`
            : ''
        }
      </div>
    `;
  }

  errorHTML += '</div>';

  overlay.innerHTML = errorHTML;

  return overlay;
}

function fullReload() {
  if ('reload' in location) {
    location.reload();
  } else if (extCtx && extCtx.runtime && extCtx.runtime.reload) {
    extCtx.runtime.reload();
  }
}

function getParents(
  // @ts-expect-error TS2304
  bundle: ParcelRequire,
  id: string,
) /*: Array<[ParcelRequire, string]> */ {
  var modules = bundle.modules;
  if (!modules) {
    return [];
  }

  // @ts-expect-error TS2304
  var parents: Array<[ParcelRequire, string]> = [];
  var k, d, dep;

  for (k in modules) {
    for (d in modules[k][1]) {
      dep = modules[k][1][d];

      if (dep === id || (Array.isArray(dep) && dep[dep.length - 1] === id)) {
        parents.push([bundle, k]);
      }
    }
  }

  if (bundle.parent) {
    parents = parents.concat(getParents(bundle.parent, id));
  }

  return parents;
}

function updateLink(link: HTMLElement) {
  var href = link.getAttribute('href');

  if (!href) {
    return;
  }
  var newLink = link.cloneNode();
  // @ts-expect-error TS2345
  newLink.onload = function () {
    if (link.parentNode !== null) {
      link.parentNode.removeChild(link);
    }
  };
  // @ts-expect-error TS2339
  newLink.setAttribute('href', href.split('?')[0] + '?' + Date.now());
  // @ts-expect-error TS18047
  link.parentNode.insertBefore(newLink, link.nextSibling);
}

// @ts-expect-error TS7034
var cssTimeout = null;
function reloadCSS() {
  // @ts-expect-error TS7005
  if (cssTimeout) {
    return;
  }

  cssTimeout = setTimeout(function () {
    var document = window.document;
    var links = document.querySelectorAll('link[rel="stylesheet"]');
    for (var i = 0; i < links.length; i++) {
      var href /*: string */ = links[i].getAttribute('href');
      var hostname = getHostname();
      var servedFromHMRServer =
        hostname === 'localhost'
          ? new RegExp(
              '^(https?:\\/\\/(0.0.0.0|127.0.0.1)|localhost):' + getPort(),
              // @ts-expect-error TS2345
            ).test(href)
          : // @ts-expect-error TS2345
            href.indexOf(hostname + ':' + getPort());
      var absolute =
        // @ts-expect-error TS2345
        /^https?:\/\//i.test(href) &&
        // @ts-expect-error TS18047
        href.indexOf(location.origin) !== 0 &&
        !servedFromHMRServer;
      if (!absolute) {
        // @ts-expect-error TS2345
        updateLink(links[i]);
      }
    }

    cssTimeout = null;
  }, 50);
}

// @ts-expect-error TS2304
function hmrDownload(asset: HMRAsset) {
  if (asset.type === 'js') {
    if (typeof document !== 'undefined') {
      let script = document.createElement('script');
      script.src = asset.url + '?t=' + Date.now();
      if (asset.outputFormat === 'esmodule') {
        script.type = 'module';
      }
      return new Promise(
        (
          resolve: (
            result: Promise<HTMLScriptElement> | HTMLScriptElement,
          ) => void,
          reject: (error?: any) => void,
        ) => {
          script.onload = () => resolve(script);
          script.onerror = reject;
          document.head?.appendChild(script);
        },
      );
      // @ts-expect-error TS2304
    } else if (typeof importScripts === 'function') {
      // Worker scripts
      if (asset.outputFormat === 'esmodule') {
        // @ts-expect-error TS2304
        return __parcel__import__(asset.url + '?t=' + Date.now());
      } else {
        return new Promise(
          (
            resolve: (result: Promise<undefined> | undefined) => void,
            reject: (error?: any) => void,
          ) => {
            try {
              // @ts-expect-error TS2304
              __parcel__importScripts__(asset.url + '?t=' + Date.now());
              // @ts-expect-error TS2794
              resolve();
            } catch (err: any) {
              reject(err);
            }
          },
        );
      }
    }
  }
}

// @ts-expect-error TS2304
async function hmrApplyUpdates(assets: Array<HMRAsset>) {
  // @ts-expect-error TS7017
  global.parcelHotUpdate = Object.create(null);

  let scriptsToRemove;
  try {
    // If sourceURL comments aren't supported in eval, we need to load
    // the update from the dev server over HTTP so that stack traces
    // are correct in errors/logs. This is much slower than eval, so
    // we only do it if needed (currently just Safari).
    // https://bugs.webkit.org/show_bug.cgi?id=137297
    // This path is also taken if a CSP disallows eval.
    if (!supportsSourceURL) {
      let promises = assets.map((asset) =>
        hmrDownload(asset)?.catch((err: any) => {
          // Web extension fix
          if (
            extCtx &&
            extCtx.runtime &&
            extCtx.runtime.getManifest().manifest_version == 3 &&
            // @ts-expect-error TS2304
            typeof ServiceWorkerGlobalScope != 'undefined' &&
            // @ts-expect-error TS2304
            global instanceof ServiceWorkerGlobalScope
          ) {
            extCtx.runtime.reload();
            return;
          }
          throw err;
        }),
      );

      scriptsToRemove = await Promise.all(promises);
    }

    assets.forEach(function (asset) {
      // @ts-expect-error TS2339
      hmrApply(module.bundle.root, asset);
    });
  } finally {
    // @ts-expect-error TS7017
    delete global.parcelHotUpdate;

    if (scriptsToRemove) {
      // @ts-expect-error TS7006
      scriptsToRemove.forEach((script) => {
        if (script) {
          document.head?.removeChild(script);
        }
      });
    }
  }
}

function hmrApply(
  // @ts-expect-error TS2304
  bundle: ParcelRequire /*: ParcelRequire */,
  // @ts-expect-error TS2304
  asset: HMRAsset /*:  HMRAsset */,
) {
  var modules = bundle.modules;
  if (!modules) {
    return;
  }

  if (asset.type === 'css') {
    reloadCSS();
  } else if (asset.type === 'js') {
    let deps = asset.depsByBundle[bundle.HMR_BUNDLE_ID];
    if (deps) {
      if (modules[asset.id]) {
        // Remove dependencies that are removed and will become orphaned.
        // This is necessary so that if the asset is added back again, the cache is gone, and we prevent a full page reload.
        let oldDeps = modules[asset.id][1];
        for (let dep in oldDeps) {
          if (!deps[dep] || deps[dep] !== oldDeps[dep]) {
            let id = oldDeps[dep];
            // @ts-expect-error TS2339
            let parents = getParents(module.bundle.root, id);
            if (parents.length === 1) {
              // @ts-expect-error TS2339
              hmrDelete(module.bundle.root, id);
            }
          }
        }
      }

      if (supportsSourceURL) {
        // Global eval. We would use `new Function` here but browser
        // support for source maps is better with eval.
        (0, eval)(asset.output);
      }

      // @ts-expect-error TS7017
      let fn = global.parcelHotUpdate[asset.id];
      modules[asset.id] = [fn, deps];
    }

    // Always traverse to the parent bundle, even if we already replaced the asset in this bundle.
    // This is required in case modules are duplicated. We need to ensure all instances have the updated code.
    if (bundle.parent) {
      hmrApply(bundle.parent, asset);
    }
  }
}

// @ts-expect-error TS2304
function hmrDelete(bundle: ParcelRequire, id: string) {
  let modules = bundle.modules;
  if (!modules) {
    return;
  }

  if (modules[id]) {
    // Collect dependencies that will become orphaned when this module is deleted.
    let deps = modules[id][1];
    let orphans: Array<string> = [];
    for (let dep in deps) {
      // @ts-expect-error TS2339
      let parents = getParents(module.bundle.root, deps[dep]);
      if (parents.length === 1) {
        orphans.push(deps[dep]);
      }
    }

    // Delete the module. This must be done before deleting dependencies in case of circular dependencies.
    delete modules[id];
    delete bundle.cache[id];

    // Now delete the orphans.
    orphans.forEach((id) => {
      // @ts-expect-error TS2339
      hmrDelete(module.bundle.root, id);
    });
  } else if (bundle.parent) {
    hmrDelete(bundle.parent, id);
  }
}

function hmrAcceptCheck(
  // @ts-expect-error TS2304
  bundle: ParcelRequire /*: ParcelRequire */,
  id: string /*: string */,
  depsByBundle:
    | {
        [key: string]: {
          [key: string]: string;
        };
      }
    | null
    | undefined /*: ?{ [string]: { [string]: string } }*/,
) {
  if (hmrAcceptCheckOne(bundle, id, depsByBundle)) {
    return true;
  }

  // Traverse parents breadth first. All possible ancestries must accept the HMR update, or we'll reload.
  // @ts-expect-error TS2339
  let parents = getParents(module.bundle.root, id);
  let accepted = false;
  while (parents.length > 0) {
    let v = parents.shift();
    // @ts-expect-error TS18048
    let a = hmrAcceptCheckOne(v[0], v[1], null);
    if (a) {
      // If this parent accepts, stop traversing upward, but still consider siblings.
      accepted = true;
    } else {
      // Otherwise, queue the parents in the next level upward.
      // @ts-expect-error TS2339
      let p = getParents(module.bundle.root, v[1]);
      if (p.length === 0) {
        // If there are no parents, then we've reached an entry without accepting. Reload.
        accepted = false;
        break;
      }
      parents.push(...p);
    }
  }

  return accepted;
}

function hmrAcceptCheckOne(
  // @ts-expect-error TS2304
  bundle: ParcelRequire /*: ParcelRequire */,
  id: string /*: string */,
  depsByBundle:
    | {
        [key: string]: {
          [key: string]: string;
        };
      }
    | null
    | undefined /*: ?{ [string]: { [string]: string } }*/,
) {
  var modules = bundle.modules;
  if (!modules) {
    return;
  }

  if (depsByBundle && !depsByBundle[bundle.HMR_BUNDLE_ID]) {
    // If we reached the root bundle without finding where the asset should go,
    // there's nothing to do. Mark as "accepted" so we don't reload the page.
    if (!bundle.parent) {
      return true;
    }

    return hmrAcceptCheck(bundle.parent, id, depsByBundle);
  }

  // @ts-expect-error TS7005
  if (checkedAssets[id]) {
    return true;
  }

  // @ts-expect-error TS7005
  checkedAssets[id] = true;

  var cached = bundle.cache[id];
  assetsToDispose.push([bundle, id]);

  if (!cached || (cached.hot && cached.hot._acceptCallbacks.length)) {
    assetsToAccept.push([bundle, id]);
    return true;
  }
}

function hmrDisposeQueue() {
  // Dispose all old assets.
  for (let i = 0; i < assetsToDispose.length; i++) {
    // @ts-expect-error TS7005
    let id = assetsToDispose[i][1];

    // @ts-expect-error TS7005
    if (!disposedAssets[id]) {
      // @ts-expect-error TS7005
      hmrDispose(assetsToDispose[i][0], id);
      disposedAssets[id] = true;
    }
  }

  assetsToDispose = [];
}

function hmrDispose(
  // @ts-expect-error TS2304
  bundle: ParcelRequire /*: ParcelRequire */,
  id: string /*: string */,
) {
  var cached = bundle.cache[id];
  bundle.hotData[id] = {};
  if (cached && cached.hot) {
    cached.hot.data = bundle.hotData[id];
  }

  if (cached && cached.hot && cached.hot._disposeCallbacks.length) {
    // @ts-expect-error TS7006
    cached.hot._disposeCallbacks.forEach(function (cb) {
      cb(bundle.hotData[id]);
    });
  }

  delete bundle.cache[id];
}

function hmrAccept(
  // @ts-expect-error TS2304
  bundle: ParcelRequire /*: ParcelRequire */,
  id: string /*: string */,
) {
  // Execute the module.
  bundle(id);

  // Run the accept callbacks in the new version of the module.
  var cached = bundle.cache[id];
  if (cached && cached.hot && cached.hot._acceptCallbacks.length) {
    let assetsToAlsoAccept: Array<never> = [];
    // @ts-expect-error TS7006
    cached.hot._acceptCallbacks.forEach(function (cb) {
      let additionalAssets = cb(function () {
        // @ts-expect-error TS2339
        return getParents(module.bundle.root, id);
      });
      if (Array.isArray(additionalAssets) && additionalAssets.length) {
        // @ts-expect-error TS2345
        assetsToAlsoAccept.push(...additionalAssets);
      }
    });

    if (assetsToAlsoAccept.length > 0) {
      let handled = assetsToAlsoAccept.every(function (a) {
        // @ts-expect-error TS2554
        return hmrAcceptCheck(a[0], a[1]);
      });

      if (!handled) {
        return fullReload();
      }

      hmrDisposeQueue();
    }
  }
}
