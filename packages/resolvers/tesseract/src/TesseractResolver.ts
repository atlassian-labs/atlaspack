import {Resolver} from '@atlaspack/plugin';
import NodeResolver from '@atlaspack/node-resolver-core';
import {basename, dirname, extname, isAbsolute, join} from 'path';
import type {EnvMap} from '@atlaspack/types-internal';

interface TesseractResolverConfig {
  /** Modules to replace with empty stubs during resolution. */
  ignoreModules?: Array<string>;

  /** Node.js built-ins to resolve using browser resolver for SnapVM compatibility. */
  browserResolvedNodeBuiltins?: Array<string>;

  /** Module mappings that bypass normal resolution (e.g., for SSR compatibility). */
  preResolved?: Record<string, string>;

  /** Node.js built-in aliases for Tesseract-specific implementations. */
  builtinAliases?: Record<string, string>;

  /** Server file suffixes checked in priority order. */
  serverSuffixes?: Array<string>;
}

// Throw user friendly errors on special webpack loader syntax
// ex. `imports-loader?$=jquery!./example.js`
const WEBPACK_IMPORT_REGEX = /\S+-loader\S*!\S+/g;

const IGNORE_PATH = join(__dirname, 'data', 'empty-module.js');

/**
 * For some of the modules that we used in static fallback html,
 * 1. we dont' want to replace it with tesseract specific version
 * 2. we want it to be resolved using browserResolver below.
 */
const STATIC_FALLBACK_MODULES = ['buffer', 'stream', 'events', 'util'];
const STATIC_FALLBACK_ALIAS: Record<string, string | undefined> = {};

const getIgnoreModules = (env: EnvMap, ignoreModules: Array<string>) => {
  if (env.PILLAR_LOCAL_DEVELOPMENT === 'true') {
    ignoreModules.push('@atlassiansox/analytics-web-client');
  }

  return ignoreModules;
};

const checkForServerFile = async (
  inputFS: any,
  resolvedPath: any,
  suffix?: any,
) => {
  const dir = dirname(resolvedPath);
  const ext = extname(resolvedPath);
  const base = basename(resolvedPath, ext);

  const serverPath = suffix
    ? join(dir, `${base}.server-${suffix}${ext}`)
    : join(dir, `${base}.server${ext}`);
  const isExist = await inputFS.exists(serverPath);
  return {
    isExist,
    serverPath,
  };
};

const checkForServerFileWithOptionalSuffixes = async (
  inputFS: any,
  resolvedPath: any,
  suffixes: any,
) => {
  if (suffixes) {
    // if there are multiple suffixes, the left-most takes precedence
    for (const suffix of suffixes) {
      const withSuffix = await checkForServerFile(
        inputFS,
        resolvedPath,
        suffix,
      );
      if (withSuffix.isExist) {
        return withSuffix;
      }
    }
  }
  return checkForServerFile(inputFS, resolvedPath);
};

export default new Resolver({
  async loadConfig({config, options, logger}) {
    // Load configuration from package.json
    const conf = await config.getConfig([], {
      packageKey: '@atlaspack/resolver-tesseract',
    });
    const userConfig: TesseractResolverConfig = conf?.contents || {};

    const preResolved = userConfig.preResolved || {};
    const builtinAliases = userConfig.builtinAliases || {};
    const serverSuffixes = userConfig.serverSuffixes || [];
    const ignoreModules = userConfig.ignoreModules || [];
    const browserResolvedNodeBuiltins =
      userConfig.browserResolvedNodeBuiltins || [];

    const nodeResolver = new NodeResolver({
      fs: options.inputFS,
      projectRoot: options.projectRoot,
      extensions: ['ts', 'tsx', 'js', 'jsx', 'json'],
      mainFields: ['source', 'module', 'main'],
      packageExports: true,
      mode: options.mode,
      logger,
    });

    const browserResolver = new NodeResolver({
      fs: options.inputFS,
      projectRoot: options.projectRoot,
      extensions: ['ts', 'tsx', 'js', 'jsx', 'json'],
      mainFields: ['browser', 'source', 'module', 'main'],
      packageExports: true,
      mode: options.mode,
      logger,
    });

    return {
      nodeResolver,
      browserResolver,
      serverSuffixes,
      preResolved,
      builtinAliases,
      ignoreModules,
      browserResolvedNodeBuiltins,
    };
  },
  resolve({dependency, specifier, config, options}) {
    // Only resolve for Tesseract environment
    if (!dependency.env.isTesseract()) {
      return null;
    }

    const {
      nodeResolver,
      browserResolver,
      ignoreModules,
      browserResolvedNodeBuiltins,
      preResolved,
      builtinAliases,
      serverSuffixes,
    } = config;

    if (WEBPACK_IMPORT_REGEX.test(dependency.specifier)) {
      throw new Error(
        `The import path: ${dependency.specifier} is using webpack specific loader import syntax, which isn't supported by Atlaspack.`,
      );
    }

    if (isAbsolute(specifier)) {
      return {
        filePath: specifier,
        code: undefined,
        sideEffects: false,
      };
    }

    if (
      getIgnoreModules(options.env, ignoreModules).some((mod) =>
        specifier.includes(mod),
      )
    ) {
      return {
        filePath: IGNORE_PATH,
        code: undefined,
        sideEffects: false,
      };
    }

    // ignore all the ./mock ./mocks modules
    if (/(mock|mocks)$/.test(specifier)) {
      return {
        filePath: IGNORE_PATH,
        code: undefined,
        sideEffects: false,
      };
    }

    // ignore all the .woff .woff2 modules
    if (/(\.woff|\.woff2)$/.test(specifier)) {
      return {
        filePath: IGNORE_PATH,
        code: undefined,
        sideEffects: false,
      };
    }

    // ignore all the .mp3 .ogg modules
    if (/(\.mp3|\.ogg)$/.test(specifier)) {
      return {
        filePath: IGNORE_PATH,
        code: undefined,
        sideEffects: false,
      };
    }

    if (preResolved[specifier]) {
      return {
        filePath: require.resolve(preResolved[specifier]),
        code: undefined,
        sideEffects: false,
      };
    }

    const aliasSpecifier =
      options.env.STATIC_FALLBACK === 'true'
        ? STATIC_FALLBACK_ALIAS[specifier]
        : builtinAliases[specifier];
    const useBrowser =
      browserResolvedNodeBuiltins.includes(specifier) ||
      (options.env.STATIC_FALLBACK === 'true' &&
        STATIC_FALLBACK_MODULES.includes(specifier));

    const snapvmEnv = new Proxy(dependency.env, {
      get(target, property) {
        if (property === 'isNode') {
          return () => false;
        }

        if (property === 'isElectron') {
          return () => false;
        }

        if (useBrowser && property === 'isLibrary') {
          return false;
        }

        const value = target[property as keyof typeof target];
        return typeof value === 'function' && value.bind
          ? value.bind(target)
          : value;
      },
    });

    const promise = useBrowser
      ? browserResolver.resolve({
          sourcePath: dependency.sourcePath,
          parent: dependency.resolveFrom,
          filename: aliasSpecifier || specifier,
          specifierType: dependency.specifierType,
          env: snapvmEnv,
          packageConditions: ['ssr', 'require'],
        })
      : nodeResolver.resolve({
          sourcePath: dependency.sourcePath,
          parent: dependency.resolveFrom,
          filename: aliasSpecifier || specifier,
          specifierType: dependency.specifierType,
          env: snapvmEnv,
          packageConditions: ['ssr', 'require'],
        });

    return promise
      .then(async (result) => {
        const resolvedPath = result?.filePath;

        if (!resolvedPath) {
          return result;
        }

        const {isExist, serverPath} =
          await checkForServerFileWithOptionalSuffixes(
            options.inputFS,
            resolvedPath,
            serverSuffixes,
          );

        if (isExist) {
          const newResult = {
            sideEffects: result.sideEffects,
            filePath: serverPath,
            meta: {
              isServerFile: true,
              resolveTo: serverPath,
            },
          };

          return newResult;
        }
        return result;
      })
      .catch((e) => {
        throw e;
      });
  },
}) as Resolver<unknown>;
