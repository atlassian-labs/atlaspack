use atlaspack_contextual_imports::ContextualImportsInlineRequireVisitor;

use crate::{Config, visitors::js_visitor::JsVisitor};

impl JsVisitor for ContextualImportsInlineRequireVisitor {
  fn should_apply(&self, config: &Config) -> bool {
    // Treat conditional imports as two inline requires when flag is off
    config.conditional_bundling
  }
}
