import {AsyncLocalStorage} from 'async_hooks';
import type {Async} from '@atlaspack/types';

export interface FsUsage {
  method: string;
  path?: string;
  stack?: string;
}

export interface EnvUsage {
  vars: Set<string>;
  didEnumerate: boolean;
}

export interface SideEffects {
  fsUsage: FsUsage[];
  envUsage: EnvUsage;
  packageName: string;
}

type OriginalMethods = Record<string, any>;

/**
 * Side effect detector using AsyncLocalStorage to track filesystem and environment variable
 * access across concurrent async operations in a Node.js worker thread.
 *
 * Usage:
 *   const detector = new SideEffectDetector();
 *   detector.install();
 *
 *   const [result, sideEffects] = await detector.monitorSideEffects(async () => {
 *     return await someOperation();
 *   });
 *
 *   console.log(sideEffects.fsUsage);   // Array of filesystem accesses
 *   console.log(sideEffects.envUsage);  // Array of environment variable accesses
 */
export class SideEffectDetector {
  private asyncStorage: AsyncLocalStorage<SideEffects>;
  private patchesInstalled: boolean;
  private originalMethods: OriginalMethods;

  constructor() {
    this.asyncStorage = new AsyncLocalStorage<SideEffects>();
    this.patchesInstalled = false;
    this.originalMethods = {};
  }

  /**
   * Install global patches for filesystem and environment variable monitoring.
   * This should be called once when the worker starts up.
   */
  install(): void {
    if (this.patchesInstalled) {
      return;
    }

    this._patchFilesystem();
    this._patchProcessEnv();

    this.patchesInstalled = true;
  }

  /**
   * Monitor side effects for an async operation.
   *
   * @param {Function} fn - Async function to monitor
   * @param {Object} options - Optional configuration
   * @param {string} options.label - Optional label for debugging
   * @returns {Promise<[any, SideEffects]>} Tuple of [result, sideEffects]
   */
  monitorSideEffects<T>(
    packageName: string,
    fn: () => Async<T>,
  ): Async<[T, SideEffects]> {
    if (!this.patchesInstalled) {
      throw new Error(
        'SideEffectDetector: install() must be called before monitorSideEffects()',
      );
    }

    const context: SideEffects = {
      fsUsage: [],
      envUsage: {
        vars: new Set(),
        didEnumerate: false,
      },
      packageName: packageName,
    };

    return this.asyncStorage.run(context, async () => {
      const result = await fn();

      return [result, context] as [T, SideEffects];
    });
  }

  /**
   * Get the current monitoring context, if any.
   * Useful for debugging or custom instrumentation.
   *
   * @returns {Object|null} Current context or null if not monitoring
   */
  getCurrentContext(): SideEffects | null {
    return this.asyncStorage.getStore() || null;
  }

  /**
   * Check if currently monitoring side effects.
   *
   * @returns {boolean}
   */
  isMonitoring(): boolean {
    return this.asyncStorage.getStore() !== undefined;
  }

  /**
   * Patch filesystem methods to record access.
   * @private
   */
  private _patchFilesystem(): void {
    // Inline require this to avoid babel transformer issue
    const fs = require('fs');
    const methodsToPatch = [
      // Sync methods
      'readFileSync',
      'writeFileSync',
      'appendFileSync',
      'existsSync',
      'statSync',
      'lstatSync',
      'readdirSync',
      'mkdirSync',
      'rmdirSync',
      'unlinkSync',
      'copyFileSync',
      'renameSync',
      'chmodSync',
      'chownSync',

      // Async methods
      'readFile',
      'writeFile',
      'appendFile',
      'stat',
      'lstat',
      'readdir',
      'mkdir',
      'rmdir',
      'unlink',
      'copyFile',
      'rename',
      'chmod',
      'chown',
    ];

    methodsToPatch.forEach((method) => {
      if (typeof fs[method] === 'function') {
        this.originalMethods[method] = fs[method];
        const self = this;

        // @ts-expect-error Dynamic method patching
        fs[method] = function (path, ...args) {
          // Record filesystem access in current context
          const context = self.asyncStorage.getStore();
          if (context) {
            const pathStr = typeof path === 'string' ? path : path?.toString();
            const fsUsage: FsUsage = {
              method,
              path: pathStr,
            };

            // Capture stack trace for package.json reads to help debug cache bailouts
            if (pathStr?.endsWith('package.json')) {
              fsUsage.stack = new Error().stack;
            }

            context.fsUsage.push(fsUsage);
          }

          return self.originalMethods[method].call(this, path, ...args);
        };
      }
    });

    // Handle fs.promises methods
    if (fs.promises) {
      const promiseMethodsToPatch = [
        'readFile',
        'writeFile',
        'appendFile',
        'stat',
        'lstat',
        'readdir',
        'mkdir',
        'rmdir',
        'unlink',
        'copyFile',
        'rename',
        'chmod',
        'chown',
      ];

      const promises = fs.promises as unknown as Record<
        string,
        (...args: any[]) => any
      >;
      promiseMethodsToPatch.forEach((method) => {
        if (typeof promises[method] === 'function') {
          const originalKey = `promises_${method}`;
          this.originalMethods[originalKey] = promises[method];
          // eslint-disable-next-line @typescript-eslint/no-this-alias
          const self = this;

          promises[method] = function (path: unknown, ...args: unknown[]) {
            const context = self.asyncStorage.getStore();
            if (context) {
              context.fsUsage.push({
                method: `promises.${method}`,
                path: typeof path === 'string' ? path : String(path),
              });
            }

            return self.originalMethods[originalKey].call(this, path, ...args);
          };
        }
      });
    }
  }

  /**
   * Patch process.env to record environment variable access.
   * @private
   */
  private _patchProcessEnv(): void {
    if (this.originalMethods.processEnv) {
      return; // Already patched
    }

    this.originalMethods.processEnv = process.env;
    // eslint-disable-next-line @typescript-eslint/no-this-alias
    const self = this;
    // The following environment variables are allowed to be accessed by transformers
    const allowedVars = new Set([
      'ATLASPACK_ENABLE_SENTRY',
      // TODO we should also add the other atlaspack env vars here
      'NODE_V8_COVERAGE',
      'VSCODE_INSPECTOR_OPTIONS',
      'NODE_INSPECTOR_IPC',
      'FORCE_COLOR',
      'NO_COLOR',
      'TTY',
    ]);

    // Create a proxy that intercepts property access
    process.env = new Proxy(this.originalMethods.processEnv, {
      get(target, property) {
        const context = self.asyncStorage.getStore();
        if (context && typeof property === 'string') {
          // Only record if this is a real environment variable access
          // (not internal properties like 'constructor', 'valueOf', etc.)
          if (
            !allowedVars.has(property) &&
            (property in target || !property.startsWith('_'))
          ) {
            context.envUsage.vars.add(property);
          }
        }
        return target[property];
      },

      set(target, property, value) {
        const context = self.asyncStorage.getStore();
        if (context && typeof property === 'string') {
          if (!allowedVars.has(property) && property in target) {
            context.envUsage.vars.add(property);
          }
        }
        target[property] = value;
        return true;
      },

      has(target, property) {
        const context = self.asyncStorage.getStore();
        if (context && typeof property === 'string') {
          if (!allowedVars.has(property) && property in target) {
            context.envUsage.vars.add(property);
          }
        }
        return property in target;
      },

      ownKeys(target) {
        const context = self.asyncStorage.getStore();
        if (context) {
          context.envUsage.didEnumerate = true;
        }
        return Object.keys(target);
      },
    });
  }
}

/**
 * Default instance for convenience. Most workers will only need one detector.
 */
export const defaultDetector = new SideEffectDetector();
