const atlaspack$moduleFactories = {};
const atlaspack$modules = {};
const atlaspack$moduleMaps = {};

function atlaspack$register(moduleId, moduleFactory, moduleMap) {
  atlaspack$moduleFactories[moduleId] = moduleFactory;
  atlaspack$moduleMaps[moduleId] = moduleMap;
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
  moduleFactory(
    exports,
    (specifier) => {
      const moduleMap = atlaspack$moduleMaps[moduleId];
      if (moduleMap[specifier]) {
        console.log("Requiring module", specifier, moduleMap[specifier]);
        const result = atlaspack$require(moduleMap[specifier]);
        console.log("  -> exports", result);
        return result;
      }

      throw new Error(`Module not found: ${specifier}`);
    },
    atlaspack$require,
    (s, value) => {
      exports[s] = value;
    },
  );

  atlaspack$modules[moduleId] = {
    exports
  };

  return exports;
}

function atlaspack$bootstrap() {
  let ms = window.atlaspack$ms;
  if (!ms) {
    ms = [];
  }

  for (let i = 0; i < ms.length; i += 1) {
    const [moduleId, moduleFactory, moduleMap] = ms[i];
    console.log("Registering module", moduleId, moduleFactory, moduleMap);
    atlaspack$register(moduleId, moduleFactory, moduleMap, ms);
  }
}
