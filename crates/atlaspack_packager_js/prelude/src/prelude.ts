type ModuleExports = Record<string, unknown>;
type Module = {exports: ModuleExports};
type ModuleFactory = (
  require: (id: string) => ModuleExports,
  module: Module,
  exports: ModuleExports,
  global: Record<string, unknown>,
) => void;

export interface AtlaspackPrelude {
  require: (id: string) => ModuleExports;
  define: (id: string, factory: ModuleFactory) => void;

  // Used for testing
  __reset?: () => void;
}

const globalObject = globalThis ?? global ?? window ?? this ?? {};
declare const MODE: 'dev' | 'prod';

let registry: Record<string, ModuleFactory> = {};
let modules: Record<string, Module> = {};
const require = (id: string): ModuleExports => {
  if (modules[id]) {
    return modules[id].exports;
  }
  const module: Module = {exports: {}};
  modules[id] = module;
  if (!registry[id]) {
    const e = new Error(`Cannot find module '${id}'`);
    // @ts-expect-error TS2334 - `code` does not exist on Error
    e.code = 'MODULE_NOT_FOUND';
    throw e;
  }
  registry[id].call(
    module.exports,
    require,
    module,
    module.exports,
    globalObject,
  );
  return module.exports;
};
const define = (id: string, factory: ModuleFactory): void => {
  registry[id] = factory;
};

// Used for testing
const __reset = MODE === 'dev' ? (): void => {
    registry = {};
    modules = {};
  } : undefined;
export default {
  require,
  define,
  __reset,
};
