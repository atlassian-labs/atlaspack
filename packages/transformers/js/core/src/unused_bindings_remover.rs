use std::collections::{HashMap, HashSet};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith, VisitWith};

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
      Pat::Rest(rest) => self.is_pattern_empty(&rest.arg),
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
    let mut prev_declaration_count = usize::MAX;

    loop {
      self.declared_bindings.clear();
      self.used_bindings.clear();

      // Collect declarations
      module.visit_with(&mut DeclarationCollector::new(&mut self.declared_bindings));

      let current_declaration_count = self.declared_bindings.len();

      // Exit if no declarations or no progress was made
      if current_declaration_count == 0 || current_declaration_count == prev_declaration_count {
        break;
      }

      // Collect usages
      module.visit_with(&mut BindingCollector::new(
        &mut self.used_bindings,
        &self.declared_bindings,
      ));

      // Remove unused bindings
      module.visit_mut_children_with(self);

      // Clean up empty declarations and imports
      self.cleanup_module_items(&mut module.body);

      prev_declaration_count = current_declaration_count;
    }
  }

  fn remove_from_pat(&self, pat: &mut Pat) {
    match pat {
      Pat::Object(obj) => {
        // Recursively process nested patterns and check for rest
        let has_rest = obj.props.iter_mut().any(|prop| match prop {
          ObjectPatProp::KeyValue(kv) => {
            self.remove_from_pat(&mut kv.value);
            false
          }
          ObjectPatProp::Rest(rest) => {
            self.remove_from_pat(&mut rest.arg);
            true
          }
          _ => false,
        });

        // Don't remove properties if rest pattern exists (affects rest contents)
        if has_rest {
          return;
        }

        // Remove unused properties
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

        // Replace unused elements with holes (None), which creates valid JS like [a, , c]
        // This preserves array positions while removing unused bindings
        for elem in &mut arr.elems {
          if matches!(elem, Some(p) if self.is_pattern_empty(p)) {
            *elem = None;
          }
        }

        // Trim trailing holes to avoid unnecessary commas like [a, , , ]
        while matches!(arr.elems.last(), Some(None)) {
          arr.elems.pop();
        }
      }
      _ => {}
    }
  }
}

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

  /// Checks if a member expression is a CommonJS export pattern.
  /// Returns true for patterns like: `module.exports.*`, `exports.*`
  fn is_cjs_export_member(&self, member: &MemberExpr) -> bool {
    match &*member.obj {
      // Pattern: exports.*
      Expr::Ident(ident) if &*ident.sym == "exports" => true,
      // Pattern: module.exports.*
      Expr::Member(inner) => matches!(
        (&*inner.obj, &inner.prop),
        (Expr::Ident(obj), MemberProp::Ident(prop))
          if &*obj.sym == "module" && &*prop.sym == "exports"
      ),
      _ => false,
    }
  }
}

impl swc_core::ecma::visit::Visit for BindingCollector<'_> {
  // Visit variable declarators to check for CJS export assignments in initializers
  fn visit_var_declarator(&mut self, declarator: &VarDeclarator) {
    // If initialized with a CJS export assignment (e.g., var bar = module.exports.x = foo),
    // mark the variable as used since it captures the result of an export with side effects
    if let Some(Expr::Assign(assign)) = declarator.init.as_deref()
      && let AssignTarget::Simple(SimpleAssignTarget::Member(member)) = &assign.left
      && self.is_cjs_export_member(member)
    {
      self.mark_bindings_in_pat_as_used(&declarator.name);
    }
    declarator.visit_children_with(self);
  }

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
    // Check if this is a CommonJS export pattern: module.exports.* = value or exports.* = value
    let is_cjs_export = match &assign.left {
      AssignTarget::Simple(SimpleAssignTarget::Member(member)) => self.is_cjs_export_member(member),
      _ => false,
    };

    match &assign.left {
      // For simple identifier assignments like foo = 1, mark the identifier as used
      AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) => {
        self.mark_binding_used(ident.id.to_id());
      }
      // For member expressions like obj.prop = value or obj[key] = value,
      // visit the entire member expression (obj and computed key if present)
      AssignTarget::Simple(SimpleAssignTarget::Member(member)) => {
        member.visit_with(self);
      }
      // For other assignment targets, use default visiting
      _ => {
        assign.left.visit_with(self);
      }
    }

    // Visit right side
    // If this is a CJS export, mark any identifiers on the right as used (exported)
    if is_cjs_export && let Expr::Ident(ident) = &*assign.right {
      self.used_bindings.insert(ident.to_id());
    }
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

  // Visit function parameters to explicitly handle default values in patterns
  fn visit_param(&mut self, param: &Param) {
    self.visit_pat(&param.pat);
  }

  // Don't visit patterns (they're declarations, not references)
  // But we need to visit default values in patterns
  fn visit_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Assign(assign) => {
        // First, visit the left side to handle nested patterns with defaults
        self.visit_pat(&assign.left);
        // Then visit the default value (right side) to collect any usages within it
        assign.right.visit_with(self);
      }
      Pat::Object(obj) => {
        // Visit computed property keys and default values in object pattern properties
        for prop in &obj.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              // Visit computed property keys like { [key]: value }
              if let PropName::Computed(computed) = &kv.key {
                computed.expr.visit_with(self);
              }
              // Visit default values in nested patterns like { b: b = foo }
              if let Pat::Assign(assign) = &*kv.value {
                assign.right.visit_with(self);
              }
            }
            ObjectPatProp::Assign(assign) => {
              // Visit default values like { foo = defaultValue }
              if let Some(value) = &assign.value {
                value.visit_with(self);
              }
            }
            _ => {}
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

  fn visit_mut_for_in_stmt(&mut self, node: &mut ForInStmt) {
    // Never visit the loop variable (node.left) since it is always necessary
    node.right.visit_mut_with(self);
    node.body.visit_mut_with(self);
  }

  fn visit_mut_for_of_stmt(&mut self, node: &mut ForOfStmt) {
    // Never visit the loop variable (node.left) since it is always necessary
    node.right.visit_mut_with(self);
    node.body.visit_mut_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  // ===========================================================================
  // Basic variable removal
  // ===========================================================================

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
  fn test_multiple_declarators() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const a = 1, b = 2;
        const unused = 1, used = 2;
        console.log(a, b, used);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = 1, b = 2;
        const used = 2;
        console.log(a, b, used);
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

  // ===========================================================================
  // Object destructuring
  // ===========================================================================

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
  fn test_handles_destructuring_alias_default() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { foo } from "foo";
        const { a, b: b = foo } = obj;
        console.log(a, b);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import { foo } from "foo";
        const { a, b: b = foo } = obj;
        console.log(a, b);
      "#}
    );
  }

  #[test]
  fn test_handles_destructuring_alias_default_constructor() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { foo } from "foo";
        class Foo {
          constructor({ a, b: b = foo } = {}) {
            this.a = a;
            this.b = b;
          }
        }
        console.log(new Foo());
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import { foo } from "foo";
        class Foo {
            constructor({ a, b: b = foo } = {}){
                this.a = a;
                this.b = b;
            }
        }
        console.log(new Foo());
      "#}
    );
  }

  #[test]
  fn test_object_rest_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, ...rest } = obj;
        const { b, ...rest2 } = obj2;
        console.log(a, rest2);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    // Cannot remove properties when rest is present - affects rest contents
    assert_eq!(
      output_code,
      indoc! {r#"
        const { a, ...rest } = obj;
        const { b, ...rest2 } = obj2;
        console.log(a, rest2);
      "#}
    );
  }

  #[test]
  fn test_object_rest_vs_no_rest() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const { a, b, ...rest } = obj;
        const { c, d, e } = obj2;
        console.log(rest, c);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { a, b, ...rest } = obj;
        const { c } = obj2;
        console.log(rest, c);
      "#}
    );
  }

  #[test]
  fn test_computed_property_exclusion_with_rest() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const key = 'excluded';
        const { [key]: _, ...rest } = obj;
        console.log(rest);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const key = 'excluded';
        const { [key]: _, ...rest } = obj;
        console.log(rest);
      "#}
    );
  }

  // ===========================================================================
  // Array destructuring
  // ===========================================================================

  #[test]
  fn test_handles_nested_array_destructuring() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, [b, c], d] = arr;
        console.log(a, c);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, [, c]] = arr;
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

    // The nested array becomes a hole to preserve d's position
    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, , d] = arr;
        console.log(a, d);
      "#}
    );
  }

  #[test]
  fn test_array_destructuring_holes_and_trimming() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, b, c] = arr;
        const [d, e, f, g] = arr2;
        console.log(a, c, d);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const [a, , c] = arr;
        const [d] = arr2;
        console.log(a, c, d);
      "#}
    );
  }

  #[test]
  fn test_array_rest_pattern() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const [a, ...rest] = arr;
        const [b, ...rest2] = arr2;
        console.log(a, rest2);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const [a] = arr;
        const [, ...rest2] = arr2;
        console.log(a, rest2);
      "#}
    );
  }

  // ===========================================================================
  // Exports
  // ===========================================================================

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
  fn test_keeps_chained_cjs_property_exports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const foo = 1;
        var bar = module.exports.default = foo;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const foo = 1;
        var bar = module.exports.default = foo;
      "#}
    );
  }

  // ===========================================================================
  // Variable usage patterns
  // ===========================================================================

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
  fn test_var_used_in_computed_access() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const index = 0;
        const key = 'foo';
        const value1 = someArray[index];
        const value2 = obj[key];
        console.log(value1, value2);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const index = 0;
        const key = 'foo';
        const value1 = someArray[index];
        const value2 = obj[key];
        console.log(value1, value2);
      "#}
    );
  }

  #[test]
  fn test_var_used_in_computed_assignment() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const obj = {};
        const arr = [];
        const key = 'foo';
        const index = 0;
        obj[key] = 'bar';
        arr[index] = 'value';
        console.log(obj, arr);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const obj = {};
        const arr = [];
        const key = 'foo';
        const index = 0;
        obj[key] = 'bar';
        arr[index] = 'value';
        console.log(obj, arr);
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
  fn test_mutation_operators() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        let count = 0;
        let i = 0;
        count += 1;
        i++;
        console.log(count, i);
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        let count = 0;
        let i = 0;
        count += 1;
        i++;
        console.log(count, i);
      "#}
    );
  }

  #[test]
  fn test_var_assign_only() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        let foo = 0;
        foo = 1;
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        let foo = 0;
        foo = 1;
      "#}
    );
  }

  // ===========================================================================
  // Scoped contexts
  // ===========================================================================

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
  fn test_unused_iteration_variable() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        for (var foo in []) {
          console.log("Hello");
        }
      "#},
      |_: RunTestContext| UnusedBindingsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        for(var foo in []){
            console.log("Hello");
        }
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

  // ===========================================================================
  // Multi-pass elimination
  // ===========================================================================

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
  fn test_multi_pass_scopes() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import used from 'used';

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
}
