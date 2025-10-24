use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith, VisitWith};

/// Helper function to recursively collect binding identifiers from patterns.
/// Calls the provided closure for each binding found, passing the identifier and export status.
fn collect_bindings_from_pat<F>(pat: &Pat, is_exported: bool, f: &mut F)
where
  F: FnMut(Id, bool),
{
  match pat {
    Pat::Ident(ident) => f(ident.id.to_id(), is_exported),
    Pat::Array(arr) => {
      for elem in arr.elems.iter().flatten() {
        collect_bindings_from_pat(elem, is_exported, f);
      }
    }
    Pat::Object(obj) => {
      for prop in &obj.props {
        match prop {
          ObjectPatProp::KeyValue(kv) => collect_bindings_from_pat(&kv.value, is_exported, f),
          ObjectPatProp::Assign(assign) => f(assign.key.to_id(), is_exported),
          ObjectPatProp::Rest(rest) => collect_bindings_from_pat(&rest.arg, is_exported, f),
        }
      }
    }
    Pat::Rest(rest) => collect_bindings_from_pat(&rest.arg, is_exported, f),
    Pat::Assign(assign) => collect_bindings_from_pat(&assign.left, is_exported, f),
    _ => {}
  }
}

/// Transformer that removes unused variable bindings.
///
/// This transform removes variable declarations that are never referenced,
/// helping to clean up dead code. It uses a multi-pass algorithm to handle
/// cascading dependencies (e.g., `const a = 1; const b = a;` where neither is used).
///
/// It handles:
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
#[derive(Default)]
pub struct UnusedBindingsRemover {
  used_bindings: HashSet<Id>,
  declared_bindings: HashMap<Id, bool>,
}

impl UnusedBindingsRemover {
  pub fn new() -> Self {
    Self::default()
  }

  fn is_special_ident(name: &str) -> bool {
    matches!(name, "di" | "jsx" | "React")
  }

  fn should_keep_binding(&self, id: &Id, is_exported: bool) -> bool {
    self.used_bindings.contains(id) || Self::is_special_ident(&id.0) || is_exported
  }

  fn is_pattern_empty(&self, pat: &Pat) -> bool {
    match pat {
      Pat::Ident(ident) => {
        let id = ident.id.to_id();
        !self.used_bindings.contains(&id) && !Self::is_special_ident(&id.0)
      }
      Pat::Object(obj) => obj.props.is_empty(),
      Pat::Array(arr) => arr.elems.iter().all(Option::is_none),
      _ => false,
    }
  }

  fn cleanup_module_items(&self, items: &mut Vec<ModuleItem>) {
    items.retain(|item| match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => !var.decls.is_empty(),
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => !import.specifiers.is_empty(),
      _ => true,
    });
  }

  fn cleanup_empty_var_decls(&self, stmts: &mut Vec<Stmt>) {
    stmts.retain(|stmt| !matches!(stmt, Stmt::Decl(Decl::Var(var)) if var.decls.is_empty()));
  }

  fn should_keep_declarator(&self, decl: &VarDeclarator) -> bool {
    match &decl.name {
      Pat::Ident(ident) => {
        let id = ident.id.to_id();
        self
          .declared_bindings
          .get(&id)
          .is_none_or(|&is_exported| self.should_keep_binding(&id, is_exported))
      }
      _ => true,
    }
  }

  fn should_keep_import_spec(&self, spec: &ImportSpecifier) -> bool {
    let id = match spec {
      ImportSpecifier::Named(named) => &named.local,
      ImportSpecifier::Default(default) => &default.local,
      ImportSpecifier::Namespace(ns) => &ns.local,
    };
    self.should_keep_binding(&id.to_id(), false)
  }

  /// Runs multiple passes of unused binding elimination until no more progress can be made.
  /// Each pass collects declarations, collects usages, then removes unused bindings.
  fn run_elimination_passes(&mut self, module: &mut Module) {
    loop {
      self.declared_bindings.clear();
      self.used_bindings.clear();

      // Collect declarations
      module.visit_with(&mut DeclarationCollector::new(&mut self.declared_bindings));

      if self.declared_bindings.is_empty() {
        break;
      }

      let declarations_before = self.declared_bindings.len();

      // Collect usages
      module.visit_with(&mut BindingCollector::new(
        &mut self.used_bindings,
        &self.declared_bindings,
      ));

      // Remove unused bindings
      module.visit_mut_children_with(self);

      // Clean up empty declarations and imports
      self.cleanup_module_items(&mut module.body);

      // Check if we made progress
      self.declared_bindings.clear();
      module.visit_with(&mut DeclarationCollector::new(&mut self.declared_bindings));

      if self.declared_bindings.len() == declarations_before {
        break;
      }
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

/// Visitor that collects all variable and import declarations.
struct DeclarationCollector<'a> {
  declared_bindings: &'a mut HashMap<Id, bool>,
}

impl<'a> DeclarationCollector<'a> {
  fn new(declared_bindings: &'a mut HashMap<Id, bool>) -> Self {
    Self { declared_bindings }
  }
}

impl swc_core::ecma::visit::Visit for DeclarationCollector<'_> {
  // Collect all variable declarations (var, let, const) - NOT function/class declarations
  fn visit_var_decl(&mut self, var: &VarDecl) {
    for declarator in &var.decls {
      self.collect_bindings_from_pat(&declarator.name, false);
    }
    // Continue visiting to find nested var decls in initializers (e.g., arrow function bodies)
    var.visit_children_with(self);
  }

  // Collect exported variable declarations
  fn visit_export_decl(&mut self, export: &ExportDecl) {
    if let Decl::Var(var) = &export.decl {
      for declarator in &var.decls {
        self.collect_bindings_from_pat(&declarator.name, true);
      }
    }
    // Continue visiting to find nested var decls
    export.visit_children_with(self);
  }

  fn visit_import_decl(&mut self, import: &ImportDecl) {
    for spec in &import.specifiers {
      let id = match spec {
        ImportSpecifier::Named(named) => &named.local,
        ImportSpecifier::Default(default) => &default.local,
        ImportSpecifier::Namespace(ns) => &ns.local,
      };
      self.declared_bindings.insert(id.to_id(), false);
    }
  }
}

impl DeclarationCollector<'_> {
  fn collect_bindings_from_pat(&mut self, pat: &Pat, is_exported: bool) {
    collect_bindings_from_pat(pat, is_exported, &mut |id, is_exp| {
      self.declared_bindings.insert(id, is_exp);
    });
  }
}

/// Visitor that collects all binding usages/references.
struct BindingCollector<'a> {
  used_bindings: &'a mut HashSet<Id>,
  declared_bindings: &'a HashMap<Id, bool>,
}

impl<'a> BindingCollector<'a> {
  fn new(used_bindings: &'a mut HashSet<Id>, declared_bindings: &'a HashMap<Id, bool>) -> Self {
    Self {
      used_bindings,
      declared_bindings,
    }
  }
}

impl BindingCollector<'_> {
  fn mark_binding_used(&mut self, id: Id) {
    if self.declared_bindings.contains_key(&id) {
      self.used_bindings.insert(id);
    }
  }
}

impl swc_core::ecma::visit::Visit for BindingCollector<'_> {
  // Visit expressions to find identifier references
  fn visit_expr(&mut self, expr: &Expr) {
    if let Expr::Ident(ident) = expr {
      self.mark_binding_used(ident.to_id());
    }
    expr.visit_children_with(self);
  }

  // Visit property shorthand: { foo } is a reference to foo
  fn visit_prop(&mut self, prop: &Prop) {
    if let Prop::Shorthand(ident) = prop {
      self.mark_binding_used(ident.to_id());
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
        self.mark_bindings_in_pat_as_used(&declarator.name);
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
    let ident = match &export.decl {
      DefaultDecl::Fn(fn_expr) => fn_expr.ident.as_ref(),
      DefaultDecl::Class(class_expr) => class_expr.ident.as_ref(),
      DefaultDecl::TsInterfaceDecl(_) => None,
    };
    if let Some(ident) = ident {
      self.used_bindings.insert(ident.to_id());
    }
    export.visit_children_with(self);
  }

  // Handle assignment expressions - visit both sides appropriately
  fn visit_assign_expr(&mut self, assign: &AssignExpr) {
    // For member expressions like obj.prop = value or obj[key] = value,
    // visit the entire member expression (obj and computed key if present)
    if let AssignTarget::Simple(SimpleAssignTarget::Member(member)) = &assign.left {
      member.visit_with(self);
    }
    // Visit right side
    assign.right.visit_with(self);
  }

  // Handle member expressions to mark computed property keys as used
  fn visit_member_expr(&mut self, member: &MemberExpr) {
    // Visit the object being accessed
    member.obj.visit_with(self);
    // For computed properties like obj[key], visit the key
    if let MemberProp::Computed(computed) = &member.prop {
      computed.expr.visit_with(self);
    }
  }

  // Don't visit patterns (they're declarations, not references)
  // But we need to visit default values in patterns
  fn visit_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Assign(assign) => {
        // Visit the default value (right side) to collect any usages within it
        assign.right.visit_with(self);
      }
      Pat::Object(obj) => {
        // Visit default values in object pattern properties
        for prop in &obj.props {
          if let ObjectPatProp::Assign(assign) = prop {
            if let Some(value) = &assign.value {
              value.visit_with(self);
            }
          }
        }
      }
      _ => {}
    }
    // Don't visit other children - patterns themselves are declarations not usages
  }
}

impl BindingCollector<'_> {
  fn mark_bindings_in_pat_as_used(&mut self, pat: &Pat) {
    collect_bindings_from_pat(pat, true, &mut |id, _| {
      self.used_bindings.insert(id);
    });
  }
}

impl VisitMut for UnusedBindingsRemover {
  fn visit_mut_module(&mut self, module: &mut Module) {
    self.run_elimination_passes(module);
  }

  fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
    node.visit_mut_children_with(self);
    self.remove_from_pat(&mut node.name);
  }

  fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
    node.visit_mut_children_with(self);
    node
      .decls
      .retain(|decl| !self.is_pattern_empty(&decl.name) && self.should_keep_declarator(decl));
  }

  fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
    block.visit_mut_children_with(self);
    self.cleanup_empty_var_decls(&mut block.stmts);
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    stmts.visit_mut_children_with(self);
    self.cleanup_empty_var_decls(stmts);
  }

  fn visit_mut_import_decl(&mut self, import: &mut ImportDecl) {
    import
      .specifiers
      .retain(|spec| self.should_keep_import_spec(spec));
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
  fn test_keeps_special_identifiers() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const React = require('react');
        const di = something;
        const jsx = other;
        const unused = 1;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const React = require('react');
        const di = something;
        const jsx = other;
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

  #[test]
  fn test_keeps_factory_in_iife_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        (function (global, factory) {
          module.exports = factory();
        })(this, (function () {
          const lottie = {};
          return lottie;
        }));
        const unused = 1;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        (function(global, factory) {
            module.exports = factory();
        })(this, (function() {
            const lottie = {};
            return lottie;
        }));
      "#}
    );
  }

  #[test]
  fn test_multi_pass_removes_cascading_unused_bindings() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const a = 1;
        const b = a;
        const c = b;
        const d = c;
        console.log('hello');
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        console.log('hello');
      "#}
    );
  }

  #[test]
  fn test_multi_pass_partial_chain() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const a = 1;
        const b = a;
        const c = b;
        const d = c;
        console.log(b);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = 1;
        const b = a;
        console.log(b);
      "#}
    );
  }

  #[test]
  fn test_removes_unused_in_function() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function foo() {
          const unused = 1;
          const used = 2;
          console.log(used);
        }
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function foo() {
            const used = 2;
            console.log(used);
        }
      "#}
    );
  }

  #[test]
  fn test_removes_unused_in_block() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function foo() {
          {
            const unused = 42;
            console.log('hello');
          }
        }
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function foo() {
            {
                console.log('hello');
            }
        }
      "#}
    );
  }

  #[test]
  fn test_unused_after_used_in_arrow() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const inner = () => {
          const c = 1;
          const unusedInner = c;
          console.log(c);
        };
        inner();
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const inner = ()=>{
            const c = 1;
            console.log(c);
        };
        inner();
      "#}
    );
  }

  #[test]
  fn test_multi_pass_scopes() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import used from 'used';
        import unused from 'unused';

        outer();

        function outer() {
          const a = 1;
          const b = a;

          const inner = () => {
            {
              const unusedBlockInner = 42;
              used();
            }

            const c = b;
            const unusedInner = c;
            console.log(c);
          };

          inner();
        }
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import used from 'used';
        outer();
        function outer() {
            const a = 1;
            const b = a;
            const inner = ()=>{
                {
                    used();
                }
                const c = b;
                console.log(c);
            };
            inner();
        }
      "#}
    );
  }

  #[test]
  fn test_removes_unused_from_object_rest_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, ...rest } = obj;
        console.log(a);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { a } = obj;
        console.log(a);
      "#}
    );
  }

  #[test]
  fn test_keeps_used_object_rest_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, ...rest } = obj;
        console.log(rest);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { ...rest } = obj;
        console.log(rest);
      "#}
    );
  }

  #[test]
  fn test_removes_unused_from_array_rest_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, ...rest] = arr;
        console.log(a);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, ...rest] = arr;
        console.log(a);
      "#}
    );
  }

  #[test]
  fn test_keeps_used_array_rest_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, ...rest] = arr;
        console.log(rest);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const [, ...rest] = arr;
        console.log(rest);
      "#}
    );
  }

  #[test]
  fn test_keeps_shorthand_property_usage() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const foo = 1;
        const unused = 2;
        const obj = { foo };
        export default obj;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const foo = 1;
        const obj = {
            foo
        };
        export default obj;
      "#}
    );
  }

  #[test]
  fn test_keeps_named_exports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const foo = 1;
        const bar = 2;
        const unused = 3;
        export { foo, bar };
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const foo = 1;
        const bar = 2;
        export { foo, bar };
      "#}
    );
  }

  #[test]
  fn test_simple_var_usage() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        var x = 1;
        console.log(x);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var x = 1;
        console.log(x);
      "#}
    );
  }

  #[test]
  fn test_var_used_in_member_assignment() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        var curve = exports;
        curve.short = require('./short');
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var curve = exports;
        curve.short = require('./short');
      "#}
    );
  }

  #[test]
  fn test_var_used_as_array_index() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const index = 0;
        const value = someArray[index];
        console.log(value);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const index = 0;
        const value = someArray[index];
        console.log(value);
      "#}
    );
  }

  #[test]
  fn test_var_used_as_computed_property() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const key = 'foo';
        const value = obj[key];
        console.log(value);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const key = 'foo';
        const value = obj[key];
        console.log(value);
      "#}
    );
  }

  #[test]
  fn test_var_property_index_assign() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const obj = {};
        const key = 'foo';
        obj[key] = 'bar';
        console.log(obj);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const obj = {};
        const key = 'foo';
        obj[key] = 'bar';
        console.log(obj);
      "#}
    );
  }

  #[test]
  fn test_var_array_index_assign() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const arr = [];
        const index = 0;
        arr[index] = 'value';
        console.log(arr);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const arr = [];
        const index = 0;
        arr[index] = 'value';
        console.log(arr);
      "#}
    );
  }

  #[test]
  fn test_var_used_in_for_loop_body() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        let retryDelay = 1000;
        for(let i = 0; i < 5; i++)retryDelay *= 2;
        console.log(retryDelay);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        let retryDelay = 1000;
        for(let i = 0; i < 5; i++)retryDelay *= 2;
        console.log(retryDelay);
      "#}
    );
  }

  #[test]
  fn test_var_used_in_compound_expression() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const r = Math.random() * 16 | 0, v = r & 0x3 | 0x8;
        return v.toString(16);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const r = Math.random() * 16 | 0, v = r & 0x3 | 0x8;
        return v.toString(16);
      "#}
    );
  }

  #[test]
  fn test_multiple_declarators_both_used() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const a = 1, b = 2;
        console.log(a, b);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = 1, b = 2;
        console.log(a, b);
      "#}
    );
  }

  #[test]
  fn test_multiple_declarators_first_unused() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const unused = 1, used = 2;
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
  fn test_compound_assignment_operator() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        let count = 0;
        count += 1;
        console.log(count);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        let count = 0;
        count += 1;
        console.log(count);
      "#}
    );
  }

  #[test]
  fn test_increment_operator() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        let i = 0;
        i++;
        console.log(i);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        let i = 0;
        i++;
        console.log(i);
      "#}
    );
  }

  #[test]
  fn test_compound_assignment_in_function() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function foo() {
            let count = 0;
            count += 1;
            console.log(count);
        }
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function foo() {
            let count = 0;
            count += 1;
            console.log(count);
        }
      "#}
    );
  }

  #[test]
  fn test_var_in_auto_function() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        (function() {
            var count = 0;
            if (true) {count = 1;}
            console.log(count);
        })();
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        (function() {
            var count = 0;
            if (true) {count = 1;}
            console.log(count);
        })();
      "#}
    );
  }

  #[test]
  fn test_var_in_destructured_function_default() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { func = function() {
            let count = 0;
            count += 1;
            console.log(count);
        } } = options;
        func();
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { func = function() {
            let count = 0;
            count += 1;
            console.log(count);
        } } = options;
        func();
      "#}
    );
  }
}
