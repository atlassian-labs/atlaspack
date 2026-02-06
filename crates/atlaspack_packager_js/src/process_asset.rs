use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::AstBuilder;
use oxc_ast::NONE;
use oxc_ast::ast::*;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType};

/// Rewrites asset code by replacing require() calls with resolved public IDs
///
/// This function:
/// 1. Parses the JavaScript code to an AST
/// 2. Traverses the AST looking for require() calls
/// 3. Replaces the specifier with the resolved public ID from the dependency map
/// 4. Generates JavaScript code back from the modified AST
///
/// # Arguments
/// * `code` - The JavaScript code to transform
/// * `deps` - Map of specifiers to their resolved public IDs (None means skipped dependency)
///
/// # Returns
/// The transformed JavaScript code with requires replaced
pub fn rewrite_asset_code(
  code: String,
  deps: &HashMap<String, Option<String>>,
) -> anyhow::Result<String> {
  let allocator = Allocator::default();
  let source_type = SourceType::default().with_module(true);

  // Parse the code
  let parser_return = Parser::new(&allocator, &code, source_type).parse();

  if !parser_return.errors.is_empty() {
    return Err(anyhow::anyhow!(
      "Failed to parse JavaScript: {:?}",
      parser_return.errors
    ));
  }

  let mut program = parser_return.program;

  // Apply the require replacement visitor
  let ast_builder = AstBuilder::new(&allocator);
  let mut visitor = RequireReplacementVisitor::new(&ast_builder, deps);
  visitor.visit_program(&mut program);

  // Generate code back from the AST
  let codegen = Codegen::new();
  let generated = codegen.build(&program);

  Ok(generated.code)
}

/// Visitor that replaces require() call specifiers with resolved public IDs
struct RequireReplacementVisitor<'a, 'alloc> {
  ast: &'a AstBuilder<'alloc>,
  deps: &'a HashMap<String, Option<String>>,
}

impl<'a, 'alloc> RequireReplacementVisitor<'a, 'alloc> {
  fn new(ast: &'a AstBuilder<'alloc>, deps: &'a HashMap<String, Option<String>>) -> Self {
    Self { ast, deps }
  }
}

impl<'a: 'alloc, 'alloc> VisitMut<'alloc> for RequireReplacementVisitor<'a, 'alloc> {
  fn visit_expression(&mut self, expr: &mut Expression<'alloc>) {
    // First recurse into children
    walk_mut::walk_expression(self, expr);

    // Replace module.bundle.root with require
    // This handles code like: module.bundle.root("id")
    if let Expression::StaticMemberExpression(member_expr) = expr
      && member_expr.property.name == "root"
      && let Expression::StaticMemberExpression(inner_member) = &member_expr.object
      && inner_member.property.name == "bundle"
      && let Expression::Identifier(ident) = &inner_member.object
      && ident.name == "module"
    {
      // Replace module.bundle.root with require identifier
      *expr = self.ast.expression_identifier(member_expr.span, "require");
      return;
    }

    // Check if this is a require() call that needs to be replaced
    if let Expression::CallExpression(call_expr) = expr
      && let Expression::Identifier(ident) = &call_expr.callee
      && ident.name == "require"
      && call_expr.arguments.len() == 1
      && let Argument::StringLiteral(string_lit) = &call_expr.arguments[0]
    {
      let specifier = string_lit.value.as_str();
      let dep_mapping = self.deps.get(specifier);

      match dep_mapping {
        Some(Some(public_id)) => {
          // Replace the string literal with the resolved public ID
          // We need to get a mutable reference to the string literal again
          if let Argument::StringLiteral(string_lit) = &mut call_expr.arguments[0] {
            string_lit.value = public_id.as_str().into();
          }
        }
        Some(None) => {
          // Replace with (function(){return {};}())

          // Create empty object: {}
          *expr = self.ast.expression_object(SPAN, self.ast.vec());
        }
        None => {
          // Keep the original specifier unchanged
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_simple_require_replacement() {
    let code = r#"const foo = require("./foo");"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_foo")"#));
  }

  #[test]
  fn test_multiple_requires() {
    let code = r#"
      const foo = require("./foo");
      const bar = require('./bar');
      const baz = require("deeply/nested/module");
    "#
    .to_string();

    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));
    deps.insert("./bar".to_string(), Some("pub_bar".to_string()));
    deps.insert(
      "deeply/nested/module".to_string(),
      Some("pub_nested".to_string()),
    );

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_foo")"#));
    assert!(result.contains(r#"require("pub_bar")"#));
    assert!(result.contains(r#"require("pub_nested")"#));
  }

  #[test]
  fn test_skipped_dependency_replaced_with_empty_object() {
    let code = r#"const foo = require("./foo"); const bar = require("./bar");"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));
    deps.insert("./bar".to_string(), None); // Skipped

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_foo")"#));
    // Skipped dependency should be replaced with (function(){return {};}())
    assert!(result.contains(r#"function() {"#) || result.contains(r#"function () {"#));
    assert!(result.contains(r#"return {};"#));
    assert!(!result.contains(r#"require("./bar")"#));
  }

  #[test]
  fn test_skipped_dependency_in_various_contexts() {
    let code = r#"
      const a = require("./a");
      const b = require("./b");
      const c = foo(require("./c"));
      const d = condition ? require("./d") : null;
    "#
    .to_string();

    let mut deps = HashMap::new();
    deps.insert("./a".to_string(), Some("pub_a".to_string()));
    deps.insert("./b".to_string(), None); // Skipped
    deps.insert("./c".to_string(), None); // Skipped
    deps.insert("./d".to_string(), None); // Skipped

    let result = rewrite_asset_code(code, &deps).unwrap();

    // Normal requires are replaced
    assert!(result.contains(r#"require("pub_a")"#));

    // All skipped dependencies are replaced with (function(){return {};}())
    // Check that we have function expressions for skipped deps
    let function_count = result.matches("function").count();
    assert!(
      function_count >= 3,
      "Expected at least 3 function expressions, found {}",
      function_count
    );

    // Make sure no skipped requires remain
    assert!(!result.contains(r#"require("./b")"#));
    assert!(!result.contains(r#"require("./c")"#));
    assert!(!result.contains(r#"require("./d")"#));
  }

  #[test]
  fn test_missing_dependency_preserved() {
    let code = r#"const foo = require("./foo"); const unknown = require("./unknown");"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));
    // ./unknown not in deps map

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_foo")"#));
    assert!(result.contains(r#"require("./unknown")"#)); // Unchanged
  }

  #[test]
  fn test_skipped_dependency_with_export_all() {
    // This simulates the real-world case where exportAll is used with a skipped dependency
    let code = r#"
      var parcelHelpers = require("@parcel/helpers");
      var _prefixerJs = require("./Prefixer.js");
      parcelHelpers.exportAll(_prefixerJs, exports);
    "#
    .to_string();

    let mut deps = HashMap::new();
    deps.insert(
      "@parcel/helpers".to_string(),
      Some("helpers_id".to_string()),
    );
    deps.insert("./Prefixer.js".to_string(), None); // Skipped

    let result = rewrite_asset_code(code, &deps).unwrap();

    // The helpers require should be replaced
    assert!(result.contains(r#"require("helpers_id")"#));

    // The skipped dependency should be replaced with a function that returns an empty object
    // This ensures that exportAll(skipped_value, exports) doesn't throw when it calls Object.keys()
    assert!(result.contains("function") && result.contains("return {};"));

    // The skipped require should not remain
    assert!(!result.contains(r#"require("./Prefixer.js")"#));
  }

  #[test]
  fn test_require_in_string_literal_not_replaced() {
    let code = r#"const str = "require('./foo')"; const foo = require("./foo");"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    // The actual require call should be replaced
    assert!(result.contains(r#"require("pub_foo")"#));
    // The string literal should NOT be modified (it should still say ./foo)
    assert!(result.contains(r#""require('./foo')""#));
  }

  #[test]
  fn test_nested_require_calls() {
    let code = r#"const result = foo(require('./bar'));"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./bar".to_string(), Some("pub_bar".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_bar")"#));
  }

  #[test]
  fn test_multiple_requires_one_line() {
    let code = r#"const a = require('./a'), b = require('./b');"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./a".to_string(), Some("pub_a".to_string()));
    deps.insert("./b".to_string(), Some("pub_b".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_a")"#));
    assert!(result.contains(r#"require("pub_b")"#));
  }

  #[test]
  fn test_require_in_if_statement() {
    let code = r#"if (condition) { const foo = require('./foo'); }"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_foo")"#));
  }

  #[test]
  fn test_require_in_return_statement() {
    let code = r#"function load() { return require('./module'); }"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./module".to_string(), Some("pub_module".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_module")"#));
  }

  #[test]
  fn test_require_in_arrow_function() {
    let code = r#"const loader = () => require('./lazy');"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./lazy".to_string(), Some("pub_lazy".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_lazy")"#));
  }

  #[test]
  fn test_require_in_ternary() {
    let code = r#"const mod = condition ? require('./a') : require('./b');"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./a".to_string(), Some("pub_a".to_string()));
    deps.insert("./b".to_string(), Some("pub_b".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_a")"#));
    assert!(result.contains(r#"require("pub_b")"#));
  }

  #[test]
  fn test_property_access_require_not_replaced() {
    let code = r#"const result = obj.require('./foo');"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    // Should NOT be replaced because it's obj.require(), not require()
    assert!(result.contains(r#"./foo"#));
    assert!(!result.contains(r#"pub_foo"#));
  }

  #[test]
  fn test_require_with_multiple_arguments_not_replaced() {
    let code = r#"const result = require('./foo', './bar');"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./foo".to_string(), Some("pub_foo".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    // Should NOT be replaced because require takes only 1 argument
    assert!(result.contains(r#"./foo"#));
    assert!(!result.contains(r#"pub_foo"#));
  }

  #[test]
  fn test_require_with_non_string_argument_not_replaced() {
    let code = r#"const result = require(variable);"#.to_string();
    let deps = HashMap::new();

    let result = rewrite_asset_code(code, &deps).unwrap();

    // Should NOT be replaced because argument is not a string literal
    assert!(result.contains(r#"require(variable)"#));
  }

  #[test]
  fn test_require_in_object_literal() {
    let code = r#"const obj = { module: require('./module') };"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./module".to_string(), Some("pub_module".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_module")"#));
  }

  #[test]
  fn test_require_in_array_literal() {
    let code = r#"const modules = [require('./a'), require('./b')];"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./a".to_string(), Some("pub_a".to_string()));
    deps.insert("./b".to_string(), Some("pub_b".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_a")"#));
    assert!(result.contains(r#"require("pub_b")"#));
  }

  #[test]
  fn test_single_and_double_quotes() {
    let code = r#"const a = require("./a"); const b = require('./b');"#.to_string();
    let mut deps = HashMap::new();
    deps.insert("./a".to_string(), Some("pub_a".to_string()));
    deps.insert("./b".to_string(), Some("pub_b".to_string()));

    let result = rewrite_asset_code(code, &deps).unwrap();

    assert!(result.contains(r#"require("pub_a")"#));
    assert!(result.contains(r#"require("pub_b")"#));
  }

  #[test]
  fn test_module_bundle_root_replacement() {
    // This simulates the runtime code that uses module.bundle.root for dynamic imports
    let code = r#"module.exports = Promise.resolve(module.bundle.root("lgIj4"));"#.to_string();
    let deps = HashMap::new();

    let result = rewrite_asset_code(code, &deps).unwrap();

    // module.bundle.root should be replaced with require
    assert!(result.contains(r#"require("lgIj4")"#));
    assert!(!result.contains("module.bundle.root"));
  }

  #[test]
  fn test_module_bundle_root_in_expression() {
    let code = r#"
      var loadBundle = function(id) {
        return module.bundle.root(id);
      };
    "#
    .to_string();
    let deps = HashMap::new();

    let result = rewrite_asset_code(code, &deps).unwrap();

    // module.bundle.root should be replaced with require
    assert!(result.contains("require(id)"));
    assert!(!result.contains("module.bundle.root"));
  }
}
