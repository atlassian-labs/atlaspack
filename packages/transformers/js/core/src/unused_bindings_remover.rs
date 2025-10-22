use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith, VisitWith};

/// Transformer that removes unused variable bindings.
///
/// This transform removes variable declarations that are never referenced,
/// helping to clean up dead code. It handles:
/// - Simple variable declarations
/// - Object destructuring patterns
/// - Array destructuring patterns
/// - Special cases like `di()` calls and exports
///
/// # Example
///
/// Input:
/// ```js
/// const unused = 1;
/// const used = 2;
/// console.log(used);
/// ```
///
/// Output:
/// ```js
/// const used = 2;
/// console.log(used);
/// ```
///
/// # Destructuring
///
/// For destructuring patterns, only unused bindings are removed:
///
/// ```js
/// const { a, b, c: { d, e } } = obj;
/// console.log(a, d);
/// ```
///
/// Output:
/// ```js
/// const { a, c: { d, } } = obj;
/// console.log(a, d);
/// ```
pub struct UnusedBindingsRemover {
  used_bindings: HashSet<Id>,
  declared_bindings: HashMap<Id, bool>,
}

impl UnusedBindingsRemover {
  pub fn new() -> Self {
    Self {
      used_bindings: HashSet::new(),
      declared_bindings: HashMap::new(),
    }
  }

  fn is_special_ident(name: &str) -> bool {
    matches!(name, "di" | "jsx" | "React")
  }

  fn should_keep_binding(&self, id: &Id, is_exported: bool) -> bool {
    self.used_bindings.contains(id) || Self::is_special_ident(&id.0) || is_exported
  }

  fn is_pattern_empty(&self, pat: &Pat) -> bool {
    match pat {
      Pat::Ident(ident) => !self.used_bindings.contains(&ident.id.to_id()),
      Pat::Object(obj) => obj.props.is_empty(),
      Pat::Array(arr) => arr.elems.iter().all(Option::is_none),
      _ => false,
    }
  }

  fn remove_from_pat(&self, pat: &mut Pat) {
    match pat {
      Pat::Object(obj) => {
        // Recursively process nested patterns
        for prop in &mut obj.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => self.remove_from_pat(&mut kv.value),
            ObjectPatProp::Rest(rest) => self.remove_from_pat(&mut rest.arg),
            _ => {}
          }
        }

        // Remove unused props
        obj.props.retain(|prop| match prop {
          ObjectPatProp::KeyValue(kv) => !self.is_pattern_empty(&kv.value),
          ObjectPatProp::Assign(assign) => self.used_bindings.contains(&assign.key.to_id()),
          ObjectPatProp::Rest(rest) => !self.is_pattern_empty(&rest.arg),
        });
      }
      Pat::Array(arr) => {
        // Recursively process nested patterns
        for elem in arr.elems.iter_mut().flatten() {
          self.remove_from_pat(elem);
        }

        // Remove unused elements
        for elem in &mut arr.elems {
          if matches!(elem, Some(p) if self.is_pattern_empty(p)) {
            *elem = None;
          }
        }
      }
      _ => {}
    }
  }
}

struct BindingCollector<'a> {
  used_bindings: &'a mut HashSet<Id>,
  declared_bindings: &'a HashMap<Id, bool>,
}

impl BindingCollector<'_> {
  fn mark_used(&mut self, id: Id) {
    if self.declared_bindings.contains_key(&id) {
      self.used_bindings.insert(id);
    }
  }
}

impl swc_core::ecma::visit::Visit for BindingCollector<'_> {
  // Visit expressions to find identifier references
  fn visit_expr(&mut self, expr: &Expr) {
    if let Expr::Ident(ident) = expr {
      self.mark_used(ident.to_id());
    }
    expr.visit_children_with(self);
  }

  // Visit property shorthand: { foo } is a reference to foo
  fn visit_prop(&mut self, prop: &Prop) {
    if let Prop::Shorthand(ident) = prop {
      self.mark_used(ident.to_id());
    }
    prop.visit_children_with(self);
  }

  // Mark exported identifiers as used (export { foo })
  fn visit_export_named_specifier(&mut self, spec: &ExportNamedSpecifier) {
    if let ModuleExportName::Ident(ident) = &spec.orig {
      self.used_bindings.insert(ident.to_id());
    }
  }

  // Mark exported declarations as used (export const foo = ...)
  fn visit_export_decl(&mut self, export: &ExportDecl) {
    if let Decl::Var(var) = &export.decl {
      for declarator in &var.decls {
        self.mark_exported_pat(&declarator.name);
      }
    }
    export.visit_children_with(self);
  }

  // Mark default exports as used (export default foo)
  fn visit_export_default_expr(&mut self, export: &ExportDefaultExpr) {
    if let Expr::Ident(ident) = &*export.expr {
      self.used_bindings.insert(ident.to_id());
    }
    export.visit_children_with(self);
  }

  // Mark default export declarations as used (export default function foo() {})
  fn visit_export_default_decl(&mut self, export: &ExportDefaultDecl) {
    match &export.decl {
      DefaultDecl::Fn(fn_expr) => {
        if let Some(ident) = &fn_expr.ident {
          self.used_bindings.insert(ident.to_id());
        }
      }
      DefaultDecl::Class(class_expr) => {
        if let Some(ident) = &class_expr.ident {
          self.used_bindings.insert(ident.to_id());
        }
      }
      DefaultDecl::TsInterfaceDecl(_) => {}
    }
    export.visit_children_with(self);
  }

  // Mark assignments to module.exports or exports.* as keeping those bindings alive
  fn visit_assign_expr(&mut self, assign: &AssignExpr) {
    // Always traverse children - this handles all expressions
    assign.visit_children_with(self);
  }

  // Don't visit patterns (they're declarations, not references)
  fn visit_pat(&mut self, _pat: &Pat) {
    // Skip - patterns are declarations not usages
  }
}

impl BindingCollector<'_> {
  fn mark_exported_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => {
        self.used_bindings.insert(ident.id.to_id());
      }
      Pat::Array(arr) => {
        for elem in arr.elems.iter().flatten() {
          self.mark_exported_pat(elem);
        }
      }
      Pat::Object(obj) => {
        for prop in &obj.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              self.mark_exported_pat(&kv.value);
            }
            ObjectPatProp::Assign(assign) => {
              self.used_bindings.insert(assign.key.to_id());
            }
            ObjectPatProp::Rest(rest) => {
              self.mark_exported_pat(&rest.arg);
            }
          }
        }
      }
      Pat::Rest(rest) => self.mark_exported_pat(&rest.arg),
      Pat::Assign(assign) => self.mark_exported_pat(&assign.left),
      _ => {}
    }
  }
}

impl VisitMut for UnusedBindingsRemover {
  fn visit_mut_module(&mut self, module: &mut Module) {
    // Collect declarations
    for item in &module.body {
      self.collect_declarations_from_module_item(item);
    }

    // Collect usages
    let mut collector = BindingCollector {
      used_bindings: &mut self.used_bindings,
      declared_bindings: &self.declared_bindings,
    };
    module.visit_with(&mut collector);

    // Remove unused bindings
    module.visit_mut_children_with(self);

    // Clean up empty declarations
    module.body.retain(
      |item| !matches!(item, ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) if var.decls.is_empty()),
    );
  }

  fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
    node.visit_mut_children_with(self);
    self.remove_from_pat(&mut node.name);
  }

  fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
    node.visit_mut_children_with(self);

    node.decls.retain(|decl| match &decl.name {
      Pat::Object(obj) if obj.props.is_empty() => false,
      Pat::Array(arr) if arr.elems.iter().all(Option::is_none) => false,
      Pat::Ident(ident) => {
        let id = ident.id.to_id();
        self
          .declared_bindings
          .get(&id)
          .is_none_or(|&is_exported| self.should_keep_binding(&id, is_exported))
      }
      _ => true,
    });
  }
}

impl UnusedBindingsRemover {
  fn collect_declarations_from_module_item(&mut self, item: &ModuleItem) {
    let (decl, is_exported) = match item {
      ModuleItem::Stmt(Stmt::Decl(decl)) => (decl, false),
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => (&export.decl, true),
      _ => return,
    };

    if let Decl::Var(var) = decl {
      for declarator in &var.decls {
        self.collect_declarations_from_pat(&declarator.name, is_exported);
      }
    }
  }

  fn collect_declarations_from_pat(&mut self, pat: &Pat, is_exported: bool) {
    match pat {
      Pat::Ident(ident) => {
        self.declared_bindings.insert(ident.id.to_id(), is_exported);
      }
      Pat::Array(arr) => {
        for elem in arr.elems.iter().flatten() {
          self.collect_declarations_from_pat(elem, is_exported);
        }
      }
      Pat::Object(obj) => {
        for prop in &obj.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              self.collect_declarations_from_pat(&kv.value, is_exported);
            }
            ObjectPatProp::Assign(assign) => {
              self
                .declared_bindings
                .insert(assign.key.to_id(), is_exported);
            }
            ObjectPatProp::Rest(rest) => {
              self.collect_declarations_from_pat(&rest.arg, is_exported);
            }
          }
        }
      }
      Pat::Rest(rest) => self.collect_declarations_from_pat(&rest.arg, is_exported),
      Pat::Assign(assign) => self.collect_declarations_from_pat(&assign.left, is_exported),
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_removes_unused_variable() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const unused = 1;
        const used = 2;
        console.log(used);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const used = 2;
        console.log(used);
      "#}
    );
  }

  #[test]
  fn test_keeps_used_variables() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const a = 1;
        const b = 2;
        console.log(a, b);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = 1;
        const b = 2;
        console.log(a, b);
      "#}
    );
  }

  #[test]
  fn test_removes_unused_from_object_destructuring() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, b, c } = obj;
        console.log(a, c);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { a, c } = obj;
        console.log(a, c);
      "#}
    );
  }

  #[test]
  fn test_removes_unused_from_array_destructuring() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, b, c] = arr;
        console.log(a, c);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, , c] = arr;
        console.log(a, c);
      "#}
    );
  }

  #[test]
  fn test_removes_entire_declaration_if_all_unused() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, b } = obj;
        console.log(other);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        console.log(other);
      "#}
    );
  }

  #[test]
  fn test_keeps_react_identifier() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const React = require('react');
        const unused = 1;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const React = require('react');
      "#}
    );
  }

  #[test]
  fn test_keeps_di_identifier() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const di = something;
        const unused = 1;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const di = something;
      "#}
    );
  }

  #[test]
  fn test_keeps_exports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        export const exported = 1;
        const notExported = 2;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export const exported = 1;
      "#}
    );
  }

  #[test]
  fn test_keeps_structured_exports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        export const { a, b } = obj;
        const { unused } = obj;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export const { a, b } = obj;
      "#}
    );
  }

  #[test]
  fn test_handles_nested_destructuring() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, b: { c, d } } = obj;
        console.log(a, c);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { a, b: { c } } = obj;
        console.log(a, c);
      "#}
    );
  }

  #[test]
  fn test_handles_deeply_nested_destructuring() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a: { b: { c, d }, e }, f } = obj;
        console.log(c, f);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { a: { b: { c } }, f } = obj;
        console.log(c, f);
      "#}
    );
  }

  #[test]
  fn test_handles_nested_array_destructuring() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, [b, c], d] = arr;
        console.log(a, c);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    // Note: The output has a trailing comma which is valid JS
    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, [, c], ] = arr;
        console.log(a, c);
      "#}
    );
  }

  #[test]
  fn test_removes_multiple_consecutive_array_elements() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, b, c, d, e] = arr;
        console.log(a, e);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    // Multiple holes in array destructuring is valid JS: [a, , , , e]
    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, , , , e] = arr;
        console.log(a, e);
      "#}
    );
  }

  #[test]
  fn test_removes_entire_nested_array_if_empty() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, [b, c], d] = arr;
        console.log(a, d);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    // The nested array [b, c] becomes empty and should be removed entirely
    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, , d] = arr;
        console.log(a, d);
      "#}
    );
  }

  #[test]
  fn test_keeps_variables_used_in_functions() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const x = 1;
        function foo() {
          return x;
        }
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const x = 1;
        function foo() {
            return x;
        }
      "#}
    );
  }

  #[test]
  fn test_keeps_default_export() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const obj = {};
        const unused = 1;
        export default obj;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const obj = {};
        export default obj;
      "#}
    );
  }

  #[test]
  fn test_keeps_default_function_export() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const unused = 1;
        export default function foo() {}
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export default function foo() {}
      "#}
    );
  }

  #[test]
  fn test_preserves_reexports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        export { default as LottiePlayer } from 'lottie-web';
        const unused = 1;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export { default as LottiePlayer } from 'lottie-web';
      "#}
    );
  }

  #[test]
  fn test_keeps_object_used_in_member_expressions() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const lottie = {};
        lottie.play = function() {};
        lottie.pause = function() {};
        const unused = 1;
        export default lottie;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const lottie = {};
        lottie.play = function() {};
        lottie.pause = function() {};
        export default lottie;
      "#}
    );
  }

  #[test]
  fn test_keeps_cjs_module_exports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const foo = 1;
        const unused = 2;
        module.exports = foo;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const foo = 1;
        module.exports = foo;
      "#}
    );
  }

  #[test]
  fn test_keeps_cjs_property_exports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const bar = 1;
        const unused = 2;
        exports.default = bar;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const bar = 1;
        exports.default = bar;
      "#}
    );
  }
}
