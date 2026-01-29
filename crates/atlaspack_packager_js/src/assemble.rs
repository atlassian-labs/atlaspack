pub fn assemble_bundle(contents: Vec<String>) -> String {
  // This is a temporary implementation that will just use string concatenation
  let prelude = r#"
  (function () {
  const registry = {};
  const modules = {};
  function define(id, factory) {
    registry[id] = factory;
  }
  function require(id) {
    if (modules[id]) {
      return modules[id].exports;
    }
    const module = { exports: {} };
    modules[id] = module;
    if (!registry[id]) {
      const e = new Error(`Module ${id} not found`);
      e.code = 'MODULE_NOT_FOUND';
      throw e;
    }
    registry[id].call(module.exports, require, module, module.exports);
    return module.exports;
  }
  "#;
  let asset_contents = contents.join("\n");
  prelude.to_string() + &asset_contents + "\n})();\n"
}
