import {Resolver} from '@atlaspack/plugin';
import NodeResolver from '@atlaspack/node-resolver-core';
import {basename, dirname, extname, isAbsolute, join} from 'path';
import {FileSystem} from '@atlaspack/types-internal';

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

const IGNORE_MODULES_REGEX = /(mock|mocks|\.woff|\.woff2|\.mp3|\.ogg)$/;

const IGNORE_PATH = join(__dirname, 'data', 'empty-module.js');

/**
 * For some of the modules that we used in static fallback html,
 * 1. we dont' want to replace it with tesseract specific version
 * 2. we want it to be resolved using browserResolver below.
 */
const STATIC_FALLBACK_MODULES = ['buffer', 'stream', 'events', 'util'];
const STATIC_FALLBACK_ALIAS: Record<string, string | undefined> = {};

const getIgnoreModules = (
  env: typeof process.env,
  ignoreModules: Array<string>,
) => {
  if (env.PILLAR_LOCAL_DEVELOPMENT === 'true') {
    return [...ignoreModules, '@atlassiansox/analytics-web-client'];
  }

  return ignoreModules;
};

async function checkForServerFile(
  inputFS: FileSystem,
  resolvedPath: string,
  suffix?: string,
) {
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
}

async function checkForServerFileWithOptionalSuffixes(
  inputFS: FileSystem,
  resolvedPath: string,
  suffixes: Array<string>,
) {
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
}

export default new Resolver({
  async loadConfig({config, options, logger}) {
    // Load configuration from package.json
    const conf = await config.getConfig([], {
      packageKey: '@atlaspack/resolver-tesseract',
    });
    const userConfig: TesseractResolverConfig = conf?.contents || {};

    const preResolved = userConfig.preResolved
      ? new Map(Object.entries(userConfig.preResolved))
      : new Map<string, string>();
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
    const {
      nodeResolver,
      browserResolver,
      ignoreModules,
      browserResolvedNodeBuiltins,
      preResolved,
      builtinAliases,
      serverSuffixes,
    } = config;

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

    // ignore mock modules and media files
    if (IGNORE_MODULES_REGEX.test(specifier)) {
      return {
        filePath: IGNORE_PATH,
        code: undefined,
        sideEffects: false,
      };
    }

    let preResolvedValue = preResolved.get(specifier);
    if (preResolvedValue) {
      const resolvedPath =
        preResolvedValue.startsWith('./') || preResolvedValue.startsWith('../')
          ? require.resolve(join(options.projectRoot, preResolvedValue))
          : require.resolve(preResolvedValue, {paths: [options.projectRoot]});
      return {
        filePath: resolvedPath,
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
        if (useBrowser && property === 'isLibrary') {
          return false;
        }

        if (typeof property === 'string') {
          const value = target[property as keyof typeof target];
          const ret = typeof value === 'function' ? value.bind(target) : value;
          return ret;
        }

        return Reflect.get(target, property);
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
