const atlaspack$moduleFactories = {};
const atlaspack$modules = {};

function atlaspack$register(moduleId, moduleFactory) {
  atlaspack$moduleFactories[moduleId] = moduleFactory;
}

function atlaspack$require(moduleId) {
  if (atlaspack$modules[moduleId]) {
    return atlaspack$modules[moduleId].exports;
  }

  const moduleFactory = atlaspack$moduleFactories[moduleId];
  if (!moduleFactory) {
    throw new Error(`Module not found: ${moduleId}`);
  }

  const exports = {};
  const module = moduleFactory(
    atlaspack$require,
    (s, value) => {
      exports[s] = value;
    },
    exports,
  );

  atlaspack$modules[moduleId] = {
    exports
  };

  return module;
}
