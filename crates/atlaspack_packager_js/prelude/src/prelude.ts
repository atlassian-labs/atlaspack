type ModuleExports = Record<string, unknown>;
type Module = {exports: ModuleExports};
type ModuleFactory = (
  require: (id: string) => ModuleExports,
  module: Module,
  exports: ModuleExports,
  global: Record<string, unknown>,
) => void;

interface AtlaspackPrelude {
  require: (id: string) => ModuleExports;
  define: (id: string, factory: ModuleFactory) => void;

  // Used for testing
  // (TODO: can we compile this out?)
  __reset: () => void;
}

const globalObject = globalThis ?? global ?? window ?? this ?? {};

let registry: Record<string, ModuleFactory> = {};
let modules: Record<string, Module> = {};
// @ts-expect-error TS2441 - require is reserved
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
const __reset = (): void => {
  registry = {};
  modules = {};
};
export default {
  require,
  define,
  __reset,
};
