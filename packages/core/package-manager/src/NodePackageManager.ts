import type {FilePath, DependencySpecifier, SemverRange} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';
import type {
  ModuleRequest,
  PackageManager,
  PackageInstaller,
  InstallOptions,
  Invalidations,
  PackageManagerResolveResult,
} from '@atlaspack/types';

import {registerSerializableClass} from '@atlaspack/build-cache';
import ThrowableDiagnostic, {
  encodeJSONKeyComponent,
  escapeMarkdown,
  generateJSONCodeHighlights,
  md,
} from '@atlaspack/diagnostic';
import {NodeFS} from '@atlaspack/fs';
import nativeFS from 'fs';
import Module from 'module';
import path from 'path';
import semver from 'semver';
import logger from '@atlaspack/logger';
import nullthrows from 'nullthrows';

import {getModuleParts} from '@atlaspack/utils';
import {getConflictingLocalDependencies} from './utils';
import {installPackage} from './installPackage';
import pkg from '../package.json';
import {getConditionsFromEnv} from './nodejsConditions';
import {ResolverBase} from '@atlaspack/node-resolver-core';
import {pathToFileURL} from 'url';
import {transformSync} from '@swc/core';

// Package.json fields. Must match package_json.rs.
const MAIN = 1 << 0;
const SOURCE = 1 << 2;
const ENTRIES =
  MAIN |
  (process.env.ATLASPACK_BUILD_ENV !== 'production' ||
  process.env.ATLASPACK_SELF_BUILD
    ? SOURCE
    : 0);

const NODE_MODULES = `${path.sep}node_modules${path.sep}`;

// There can be more than one instance of NodePackageManager, but node has only a single module cache.
// Therefore, the resolution cache and the map of parent to child modules should also be global.
const cache = new Map<DependencySpecifier, PackageManagerResolveResult>();
const children = new Map<FilePath, Set<DependencySpecifier>>();
const invalidationsCache = new Map<string, Invalidations>();

// This implements a package manager for Node by monkey patching the Node require
// algorithm so that it uses the specified FileSystem instead of the native one.
// It also handles installing packages when they are required if not already installed.
// See https://github.com/nodejs/node/blob/master/lib/internal/modules/cjs/loader.js
// for reference to Node internals.
export class NodePackageManager implements PackageManager {
  fs: FileSystem;
  projectRoot: FilePath;
  installer: PackageInstaller | null | undefined;
  resolver: ResolverBase;
  currentExtensions: Array<string>;

  constructor(
    fs: FileSystem,
    projectRoot: FilePath,
    installer?: PackageInstaller | null,
  ) {
    this.fs = fs;
    this.projectRoot = projectRoot;
    this.installer = installer;

    this.currentExtensions = Object.keys(Module._extensions).map((e) =>
      e.substring(1),
    );
  }

  _createResolver(): ResolverBase {
    return new ResolverBase(this.projectRoot, {
      fs:
        this.fs instanceof NodeFS && process.versions.pnp == null
          ? undefined
          : {
              canonicalize: (path) => this.fs.realpathSync(path),
              read: (path) => this.fs.readFileSync(path),
              isFile: (path) => this.fs.statSync(path).isFile(),
              isDir: (path) => this.fs.statSync(path).isDirectory(),
            },
      mode: 2,
      entries: ENTRIES,
      packageExports: true,
      moduleDirResolver:
        process.versions.pnp != null
          ? (module: any, from: any) => {
              let pnp = Module.findPnpApi(path.dirname(from));

              return pnp.resolveToUnqualified(
                // append slash to force loading builtins from npm
                module + '/',
                from,
              );
            }
          : undefined,
      extensions: this.currentExtensions,
      typescript: true,
    });
  }

  static deserialize(opts: any): NodePackageManager {
    return new NodePackageManager(opts.fs, opts.projectRoot, opts.installer);
  }

  serialize(): {
    $$raw: boolean;
    fs: FileSystem;
    projectRoot: FilePath;
    installer: PackageInstaller | null | undefined;
  } {
    return {
      $$raw: false,
      fs: this.fs,
      projectRoot: this.projectRoot,
      installer: this.installer,
    };
  }

  async require(
    name: DependencySpecifier,
    from: FilePath,
    opts?: {
      range?: SemverRange | null | undefined;
      shouldAutoInstall?: boolean;
      saveDev?: boolean;
    } | null,
  ): Promise<any> {
    let {resolved, type} = await this.resolve(name, from, opts);
    if (type === 2) {
      logger.warn({
        message: 'ES module dependencies are experimental.',
        origin: '@atlaspack/package-manager',
        codeFrames: [
          {
            filePath: resolved,
            codeHighlights: [],
          },
        ],
      });

      // On Windows, Node requires absolute paths to be file URLs.
      if (process.platform === 'win32' && path.isAbsolute(resolved)) {
        resolved = pathToFileURL(resolved);
      }

      return import(resolved);
    }
    return this.load(resolved, from);
  }

  requireSync(name: DependencySpecifier, from: FilePath): any {
    let {resolved} = this.resolveSync(name, from);
    return this.load(resolved, from);
  }

  load(filePath: FilePath, from: FilePath): any {
    if (!path.isAbsolute(filePath)) {
      // Node builtin module
      return require(filePath);
    }

    const cachedModule = Module._cache[filePath];
    if (cachedModule !== undefined) {
      return cachedModule.exports;
    }

    let m = new Module(filePath, Module._cache[from] || module.parent);

    const extensions = Object.keys(Module._extensions);
    // This handles supported extensions changing due to, for example, esbuild/register being used
    // We assume that the extension list will change in size - as these tools usually add support for
    // additional extensions.
    if (extensions.length !== this.currentExtensions.length) {
      this.currentExtensions = extensions.map((e) => e.substring(1));
      this.resolver = this._createResolver();
    }

    Module._cache[filePath] = m;

    // Patch require within this module so it goes through our require
    m.require = (id: any) => {
      return this.requireSync(id, filePath);
    };

    // Patch `fs.readFileSync` temporarily so that it goes through our file system
    let {readFileSync, statSync} = nativeFS;
    nativeFS.readFileSync = (filename: any, encoding: any) => {
      return this.fs.readFileSync(filename, encoding);
    };

    nativeFS.statSync = (filename: any) => {
      return this.fs.statSync(filename);
    };

    if (!filePath.includes(NODE_MODULES)) {
      let extname = path.extname(filePath);
      if (
        (extname === '.ts' ||
          extname === '.tsx' ||
          extname === '.mts' ||
          extname === '.cts') &&
        !Module._extensions[extname]
      ) {
        let compile = m._compile;
        m._compile = (code: any, filename: any) => {
          let out = transformSync(code, {filename, module: {type: 'commonjs'}});
          compile.call(m, out.code, filename);
        };

        Module._extensions[extname] = (m: any, filename: any) => {
          delete Module._extensions[extname];
          Module._extensions['.js'](m, filename);
        };
      }
    }

    try {
      m.load(filePath);
    } catch (err: any) {
      delete Module._cache[filePath];
      throw err;
    } finally {
      nativeFS.readFileSync = readFileSync;
      nativeFS.statSync = statSync;
    }

    return m.exports;
  }

  async resolve(
    id: DependencySpecifier,
    from: FilePath,
    options?: {
      range?: SemverRange | null | undefined;
      shouldAutoInstall?: boolean;
      saveDev?: boolean;
    } | null,
  ): Promise<PackageManagerResolveResult> {
    let basedir = path.dirname(from);
    let key = basedir + ':' + id;
    let resolved = cache.get(key);
    if (!resolved) {
      let [name] = getModuleParts(id);
      try {
        resolved = this.resolveInternal(id, from);
      } catch (e: any) {
        if (
          e.code !== 'MODULE_NOT_FOUND' ||
          options?.shouldAutoInstall !== true ||
          id.startsWith('.') // a local file, don't autoinstall
        ) {
          if (
            e.code === 'MODULE_NOT_FOUND' &&
            options?.shouldAutoInstall !== true
          ) {
            let err = new ThrowableDiagnostic({
              diagnostic: {
                message: escapeMarkdown(e.message),
                hints: [
                  'Autoinstall is disabled, please install this package manually and restart Parcel.',
                ],
              },
            });
            err.code = 'MODULE_NOT_FOUND';
            throw err;
          } else {
            throw e;
          }
        }

        let conflicts = await getConflictingLocalDependencies(
          this.fs,
          name,
          from,
          this.projectRoot,
        );

        if (conflicts == null) {
          this.invalidate(id, from);
          await this.install([{name, range: options?.range}], from, {
            saveDev: options?.saveDev ?? true,
          });

          return this.resolve(id, from, {
            ...options,
            shouldAutoInstall: false,
          });
        }

        throw new ThrowableDiagnostic({
          diagnostic: conflicts.fields.map((field) => ({
            message: md`Could not find module "${name}", but it was listed in package.json. Run your package manager first.`,
            origin: '@atlaspack/package-manager',
            codeFrames: [
              {
                filePath: conflicts.filePath,
                language: 'json',
                code: conflicts.json,
                codeHighlights: generateJSONCodeHighlights(conflicts.json, [
                  {
                    key: `/${field}/${encodeJSONKeyComponent(name)}`,
                    type: 'key',
                    message: 'Defined here, but not installed',
                  },
                ]),
              },
            ],
          })),
        });
      }

      let range = options?.range;
      if (range != null) {
        let pkg = resolved.pkg;
        if (pkg == null || !semver.satisfies(pkg.version, range)) {
          let conflicts = await getConflictingLocalDependencies(
            this.fs,
            name,
            from,
            this.projectRoot,
          );

          if (conflicts == null && options?.shouldAutoInstall === true) {
            this.invalidate(id, from);
            await this.install([{name, range}], from);
            return this.resolve(id, from, {
              ...options,
              shouldAutoInstall: false,
            });
          } else if (conflicts != null) {
            throw new ThrowableDiagnostic({
              diagnostic: {
                message: md`Could not find module "${name}" satisfying ${range}.`,
                origin: '@atlaspack/package-manager',
                codeFrames: [
                  {
                    filePath: conflicts.filePath,
                    language: 'json',
                    code: conflicts.json,
                    codeHighlights: generateJSONCodeHighlights(
                      conflicts.json,
                      conflicts.fields.map((field) => ({
                        key: `/${field}/${encodeJSONKeyComponent(name)}`,
                        type: 'key',
                        message: 'Found this conflicting local requirement.',
                      })),
                    ),
                  },
                ],
              },
            });
          }

          let version = pkg?.version;
          let message = md`Could not resolve package "${name}" that satisfies ${range}.`;
          if (version != null) {
            message += md` Found ${version}.`;
          }

          throw new ThrowableDiagnostic({
            diagnostic: {
              message,
              hints: [
                'Looks like the incompatible version was installed transitively. Add this package as a direct dependency with a compatible version range.',
              ],
            },
          });
        }
      }

      cache.set(key, resolved);
      invalidationsCache.clear();

      // Add the specifier as a child to the parent module.
      // Don't do this if the specifier was an absolute path, as this was likely a dynamically resolved path
      // (e.g. babel uses require() to load .babelrc.js configs and we don't want them to be added  as children of babel itself).
      if (!path.isAbsolute(name)) {
        let moduleChildren = children.get(from);
        if (!moduleChildren) {
          moduleChildren = new Set();
          children.set(from, moduleChildren);
        }

        moduleChildren.add(name);
      }
    }

    return resolved;
  }

  resolveSync(name: DependencySpecifier, from: FilePath): PackageManagerResolveResult {
    let basedir = path.dirname(from);
    let key = basedir + ':' + name;
    let resolved = cache.get(key);
    if (!resolved) {
      resolved = this.resolveInternal(name, from);
      cache.set(key, resolved);
      invalidationsCache.clear();

      if (!path.isAbsolute(name)) {
        let moduleChildren = children.get(from);
        if (!moduleChildren) {
          moduleChildren = new Set();
          children.set(from, moduleChildren);
        }

        moduleChildren.add(name);
      }
    }

    return resolved;
  }

  async install(
    modules: Array<ModuleRequest>,
    from: FilePath,
    opts?: InstallOptions,
  ) {
    await installPackage(this.fs, this, modules, from, this.projectRoot, {
      packageInstaller: this.installer,
      ...opts,
    });
  }

  getInvalidations(name: DependencySpecifier, from: FilePath): Invalidations {
    let basedir = path.dirname(from);
    let cacheKey = basedir + ':' + name;
    let resolved = cache.get(cacheKey);

    if (resolved && path.isAbsolute(resolved.resolved)) {
      let cached = invalidationsCache.get(resolved.resolved);
      if (cached != null) {
        return cached;
      }

      let res = {
        invalidateOnFileCreate: [],
        invalidateOnFileChange: new Set(),
        invalidateOnStartup: false,
      };

      let seen = new Set();
      let addKey = (name: DependencySpecifier, from: FilePath | DependencySpecifier) => {
        let basedir = path.dirname(from);
        let key = basedir + ':' + name;
        if (seen.has(key)) {
          return;
        }

        seen.add(key);
        let resolved = cache.get(key);
        if (!resolved || !path.isAbsolute(resolved.resolved)) {
          return;
        }

        res.invalidateOnFileCreate.push(...resolved.invalidateOnFileCreate);
        res.invalidateOnFileChange.add(resolved.resolved);

        for (let file of resolved.invalidateOnFileChange) {
          res.invalidateOnFileChange.add(file);
        }

        let moduleChildren = children.get(resolved.resolved);
        if (moduleChildren) {
          for (let specifier of moduleChildren) {
            addKey(specifier, resolved.resolved);
          }
        }
      };

      addKey(name, from);

      // If this is an ES module, we won't have any of the dependencies because import statements
      // cannot be intercepted. Instead, ask the resolver to parse the file and recursively analyze the deps.
      if (resolved.type === 2) {
        let invalidations = this.resolver.getInvalidations(resolved.resolved);
        invalidations.invalidateOnFileChange.forEach((i) =>
          res.invalidateOnFileChange.add(i),
        );
        invalidations.invalidateOnFileCreate.forEach((i) =>
          res.invalidateOnFileCreate.push(i),
        );
        res.invalidateOnStartup ||= invalidations.invalidateOnStartup;
        if (res.invalidateOnStartup) {
          logger.warn({
            message: md`${path.relative(
              this.projectRoot,
              resolved.resolved,
            )} contains non-statically analyzable dependencies in its module graph. This causes Parcel to invalidate the cache on startup.`,
            origin: '@atlaspack/package-manager',
          });
        }
      }

      invalidationsCache.set(resolved.resolved, res);
      return res;
    }

    return {
      invalidateOnFileCreate: [],
      invalidateOnFileChange: new Set(),
      invalidateOnStartup: false,
    };
  }

  invalidate(name: DependencySpecifier, from: FilePath) {
    let seen = new Set();

    let invalidate = (name: DependencySpecifier, from: FilePath | DependencySpecifier) => {
      let basedir = path.dirname(from);
      let key = basedir + ':' + name;
      if (seen.has(key)) {
        return;
      }

      seen.add(key);
      let resolved = cache.get(key);
      if (!resolved || !path.isAbsolute(resolved.resolved)) {
        return;
      }

      invalidationsCache.delete(resolved.resolved);

      let module = Module._cache[resolved.resolved];
      if (module) {
        delete Module._cache[resolved.resolved];
      }

      let moduleChildren = children.get(resolved.resolved);
      if (moduleChildren) {
        for (let specifier of moduleChildren) {
          invalidate(specifier, resolved.resolved);
        }
      }

      children.delete(resolved.resolved);
      cache.delete(key);
    };

    invalidate(name, from);
    this.resolver = this._createResolver();
  }

  resolveInternal(name: string, from: string): PackageManagerResolveResult {
    if (this.resolver == null) {
      this.resolver = this._createResolver();
    }

    let res = this.resolver.resolve({
      filename: name,
      specifierType: 'commonjs',
      parent: from,
      packageConditions: getConditionsFromEnv(),
    });

    // Invalidate whenever the .pnp.js file changes.
    // TODO: only when we actually resolve a node_modules package?
    if (process.versions.pnp != null && res.invalidateOnFileChange) {
      let pnp = Module.findPnpApi(path.dirname(from));
      res.invalidateOnFileChange.push(pnp.resolveToUnqualified('pnpapi', null));
    }

    if (res.error) {
      let e = new Error(`Could not resolve module "${name}" from "${from}"`);
      e.code = 'MODULE_NOT_FOUND';
      throw e;
    }
    let getPkg;
    switch (res.resolution.type) {
      case 'Path':
        getPkg = () => {
          let pkgPath = this.fs.findAncestorFile(
            ['package.json'],
            nullthrows(res.resolution.value),
            this.projectRoot,
          );
          return pkgPath
            ? JSON.parse(this.fs.readFileSync(pkgPath, 'utf8'))
            : null;
        };
      // fallthrough
      case 'Builtin':
        return {
          resolved: res.resolution.value,
          invalidateOnFileChange: new Set(res.invalidateOnFileChange),
          invalidateOnFileCreate: res.invalidateOnFileCreate,
          type: res.moduleType,
          get pkg() {
            return getPkg();
          },
        };
      default:
        throw new Error('Unknown resolution type');
    }
  }
}

registerSerializableClass(
  `${pkg.version}:NodePackageManager`,
  NodePackageManager,
);
