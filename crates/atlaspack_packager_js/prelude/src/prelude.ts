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
declare const MODE: 'debug' | 'dev' | 'prod';

let registry: Record<string, ModuleFactory> = {};
let modules: Record<string, Module> = {};
let logEntries: string[] = MODE === 'debug' ? [] : undefined as unknown as string[];

// Debug-only: tracks the current require call chain
const requireStack: string[] = MODE === 'debug' ? [] : (undefined as unknown as string[]);
const log: (message: string) => void = MODE === 'debug' ? (message: string) => logEntries.push(message) : () => {};
const metrics: { registered: number, executed: number, timings: Record<string, number> } = { registered: 0, executed: 0, timings: {} };
const require = (id: string): ModuleExports => {
  if (modules[id]) {
    MODE === 'debug' && log(`${' '.repeat(requireStack.length)}require(${id})`);
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
  if (MODE === 'debug') {
    requireStack.push(id);
    const startTime = Date.now();
    try {
      registry[id].call(
        module.exports,
        require,
        module,
        module.exports,
        globalObject,
      );
      const endTime = Date.now();
      log(`${' '.repeat(requireStack.length - 1)}require(${id})::factory ${endTime - startTime}ms`);
      metrics.timings[id] = endTime - startTime;
    } catch (e) {
      log(`\nRequire stack trace (module that threw listed first):`);
      for (let i = requireStack.length - 1; i >= 0; i--) {
        log(`  ${i === requireStack.length - 1 ? '>' : ' '} ${requireStack[i]}`);
      }
      throw e;
    } finally {
      requireStack.pop();
    }
  } else {
    registry[id].call(
      module.exports,
      require,
      module,
      module.exports,
      globalObject,
    );
  }
  metrics.executed += 1;
  return module.exports;
};
const define = (id: string, factory: ModuleFactory): void => {
  registry[id] = factory;
  metrics.registered += 1;
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
  metrics,
  logEntries
};
