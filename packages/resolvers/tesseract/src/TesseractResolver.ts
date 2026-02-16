import {Resolver} from '@atlaspack/plugin';
import NodeResolver from '@atlaspack/node-resolver-core';
import {isAbsolute, join} from 'path';

interface TesseractResolverConfig {
  /** Modules to replace with empty stubs during resolution. */
  ignoreModules?: Array<string>;

  /** Node.js built-ins to resolve using browser resolver for SnapVM compatibility. */
  browserResolvedNodeBuiltins?: Array<string>;

  /** Module mappings that bypass normal resolution (e.g., for SSR compatibility). */
  preResolved?: Record<string, string>;

  /** Node.js built-in aliases for Tesseract-specific implementations. */
  builtinAliases?: Record<string, string>;

  /** Enable React DOM Server specific behavior. */
  handleReactDomServer?: boolean;

  /** Unsupported extensions, configure to fallback to default behaviour */
  unsupportedExtensions?: Array<string>;
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
  if (env.SSR_IGNORE_MODULES) {
    const additionalIgnoreModules = env.SSR_IGNORE_MODULES.split(',')
      .map((module) => module.trim())
      .filter((module) => module.length > 0);
    return [...ignoreModules, ...additionalIgnoreModules];
  }
  return ignoreModules;
};

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
    const ignoreModules = userConfig.ignoreModules || [];
    const browserResolvedNodeBuiltins =
      userConfig.browserResolvedNodeBuiltins || [];
    const handleReactDomServer = userConfig.handleReactDomServer || false;
    const unsupportedExtensions = userConfig.unsupportedExtensions || [];
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
      preResolved,
      builtinAliases,
      ignoreModules,
      browserResolvedNodeBuiltins,
      handleReactDomServer,
      unsupportedExtensions,
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
      handleReactDomServer,
      unsupportedExtensions,
    } = config;

    if (
      unsupportedExtensions.some((ext) => dependency.specifier.endsWith(ext))
    ) {
      // fallback to atlaspack's default resolver
      return null;
    }

    if (!specifier.startsWith('//') && isAbsolute(specifier)) {
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
        if (handleReactDomServer && specifier.includes('react-dom/server')) {
          if (property === 'isNode') {
            return () => true;
          }
          if (property === 'isBrowser') {
            return () => false;
          }
          if (property === 'isWorker') {
            return () => false;
          }
        }

        if (property === 'isLibrary') {
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

    const packageConditions =
      handleReactDomServer && specifier.includes('react-dom/server')
        ? ['default']
        : ['ssr', 'require'];

    const promise = useBrowser
      ? browserResolver.resolve({
          sourcePath: dependency.sourcePath,
          parent: dependency.resolveFrom,
          filename: aliasSpecifier || specifier,
          specifierType: dependency.specifierType,
          env: snapvmEnv,
          packageConditions,
        })
      : nodeResolver.resolve({
          sourcePath: dependency.sourcePath,
          parent: dependency.resolveFrom,
          filename: aliasSpecifier || specifier,
          specifierType: dependency.specifierType,
          env: handleReactDomServer ? snapvmEnv : dependency.env,
          packageConditions,
        });

    return promise;
  },
}) as Resolver<unknown>;
