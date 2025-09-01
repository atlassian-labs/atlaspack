use std::collections::{HashMap, HashSet};

use swc_core::ecma::ast::*;
use swc_core::ecma::utils::ident::IdentLike;
use swc_core::ecma::visit::{Visit, VisitWith};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ExportKind {
  Const,
  Let,
  Var,
  Function,
  Class,
  // Literal,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SymbolInfo {
  pub export_kind: ExportKind,
  pub is_reassigned: bool,
}

pub struct ExportScannerVisitor<'a> {
  // Keep track of exported idenifiers (values are the export names)
  exported_identifiers: HashMap<Id, HashSet<Id>>,
  reassignments: HashSet<Id>,
  symbol_info: &'a mut HashMap<Id, SymbolInfo>,
}

fn export_kind_from_decl(decl: &VarDecl) -> ExportKind {
  match decl.kind {
    VarDeclKind::Const => ExportKind::Const,
    VarDeclKind::Let => ExportKind::Let,
    VarDeclKind::Var => ExportKind::Var,
  }
}

impl<'a> ExportScannerVisitor<'a> {
  fn new(symbol_info: &'a mut HashMap<Id, SymbolInfo>) -> ExportScannerVisitor<'a> {
    Self {
      exported_identifiers: HashMap::new(),
      reassignments: HashSet::new(),
      symbol_info,
    }
  }

  fn find_exports_from_var_decl(&mut self, var: &VarDecl) {
    let export_kind = export_kind_from_decl(var);
    for decl in &var.decls {
      match &decl.name {
        Pat::Ident(binding_ident) => {
          self.find_exports_from_binding_ident(binding_ident, &export_kind)
        }
        Pat::Array(array_pat) => self.find_exports_from_array_pattern(array_pat, &export_kind),
        Pat::Rest(rest_pat) => self.find_exports_from_rest_pattern(rest_pat, &export_kind),
        Pat::Object(object_pat) => {
          self.find_exports_from_object_pattern(object_pat, &export_kind);
        }
        Pat::Assign(assign_pat) => self.find_exports_from_assign_pattern(assign_pat, &export_kind),
        Pat::Invalid(_) => {}
        Pat::Expr(_) => {}
      }
    }
  }

  /// Extracts exports from a binding ident.
  ///
  /// For example:
  ///
  ///     export const foo = 1;
  ///
  /// Will extract:
  ///
  ///     const foo = 1;
  ///     exports.foo = foo;
  ///
  fn find_exports_from_binding_ident(
    &mut self,
    binding_ident: &BindingIdent,
    export_kind: &ExportKind,
  ) {
    let ident = binding_ident.id.clone();
    self.symbol_info.insert(
      ident.to_id(),
      SymbolInfo {
        export_kind: *export_kind,
        is_reassigned: false,
      },
    );
  }

  /// Extracts exports from an object pattern.
  ///
  /// For example:
  ///
  ///     export const { foo } = obj;
  ///
  /// Will extract:
  ///
  ///     const { foo } = obj;
  ///     exports.foo = foo;
  ///
  fn find_exports_from_object_pattern(&mut self, object_pat: &ObjectPat, export_kind: &ExportKind) {
    for prop in &object_pat.props {
      match prop {
        // This is `foo` in:
        // { foo }
        ObjectPatProp::Assign(prop) => {
          let key = prop.key.clone();
          assert!(prop.value.is_none());
          self.symbol_info.insert(
            key.to_id(),
            SymbolInfo {
              export_kind: *export_kind,
              is_reassigned: false,
            },
          );
        }
        // This is `foo` in:
        // { prop: foo }
        ObjectPatProp::KeyValue(prop) => {
          let value = &prop.value;
          self.find_exports_from_pat(value, export_kind);
        }
        // This is `foo` in:
        // { ...foo }
        ObjectPatProp::Rest(rest_pat) => {
          self.find_exports_from_rest_pattern(rest_pat, export_kind);
        }
      }
    }
  }

  /// Extracts exports from a pattern.
  ///
  /// This happens when we have:
  ///
  ///     export const { foo: <PAT> } = ...;
  ///
  /// The issue is there are many valid patterns. For example:
  ///
  ///     export const { foo: { bar } } = ...;
  ///
  fn find_exports_from_pat(&mut self, pat: &Pat, export_kind: &ExportKind) {
    match pat {
      Pat::Ident(binding_ident) => {
        self.find_exports_from_binding_ident(binding_ident, export_kind);
      }
      Pat::Array(array_pat) => {
        self.find_exports_from_array_pattern(array_pat, export_kind);
      }
      Pat::Object(object_pat) => {
        self.find_exports_from_object_pattern(object_pat, export_kind);
      }
      Pat::Rest(rest_pat) => {
        self.find_exports_from_rest_pattern(rest_pat, export_kind);
      }
      Pat::Assign(assign_pat) => {
        self.find_exports_from_assign_pattern(assign_pat, export_kind);
      }
      // These cases are INVALID
      // Pat expr is for for-in/for-of loops.
      Pat::Expr(_) => {}
      Pat::Invalid(_) => {}
    }
  }

  /// Extracts exports from a pattern.
  ///
  /// This happens when we have:
  ///
  ///     export const [<PAT>]= ...;
  ///
  /// The issue is there are many valid patterns. For example:
  ///
  ///     export const [foo, bar, ...rest, { abc }] = ...;
  ///
  /// We recursively extract exports from each element. Missing elements are ignored.
  /// An element is missing on this sample:
  ///
  ///     export const [one, two, , four] = ...;
  ///
  fn find_exports_from_array_pattern(&mut self, array_pat: &ArrayPat, export_kind: &ExportKind) {
    for elem in array_pat.elems.iter().flatten() {
      self.find_exports_from_pat(elem, export_kind);
    }
  }

  /// Extracts exports from a rest pattern.
  ///
  /// This happens when we have:
  ///
  ///     export const { ...<PAT> } = ...;
  ///     export const [ ...<PAT> ] = ...;
  ///
  /// We recursively extract exports from the `<PAT>` node.
  fn find_exports_from_rest_pattern(&mut self, rest_pat: &RestPat, export_kind: &ExportKind) {
    self.find_exports_from_pat(&rest_pat.arg, export_kind);
  }

  /// Extracts exports from an assign pattern.
  ///
  /// This happens when we have:
  ///
  ///     export const { a = 10 } = ...;
  ///     export const { <PAT> = <DEFAULT> } = ...;
  ///
  /// We recursively extract exports from `<PAT>`.
  fn find_exports_from_assign_pattern(&mut self, assign_pat: &AssignPat, export_kind: &ExportKind) {
    self.find_exports_from_pat(&assign_pat.left, export_kind);
  }

  fn find_exports_from_expr(&mut self, expr: &Expr) {
    if let Expr::Ident(ident) = &expr {
      self
        .exported_identifiers
        .entry(ident.to_id())
        .or_default()
        .insert(ident.to_id());
    }
  }

  fn find_exports_from_specifier(&mut self, specifier: &ExportSpecifier) {
    if let ExportSpecifier::Named(named_specifier) = specifier {
      if let ModuleExportName::Ident(orig_ident) = named_specifier.orig.clone() {
        let exported: ModuleExportName = named_specifier
          .exported
          .clone()
          .unwrap_or(ModuleExportName::Ident(orig_ident.clone()));
        if let ModuleExportName::Ident(exported_ident) = exported {
          self
            .exported_identifiers
            .entry(orig_ident.to_id())
            .or_default()
            .insert(exported_ident.to_id());
        }
      }
    }
  }

  fn find_exports_from_function_decl(&mut self, func: &FnDecl) {
    self.symbol_info.insert(
      func.ident.to_id(),
      SymbolInfo {
        export_kind: ExportKind::Function,
        is_reassigned: false, // If we're here, this function was declared inside an export
      },
    );
  }

  fn find_exports_from_class_decl(&mut self, class: &ClassDecl) {
    self.symbol_info.insert(
      class.ident.to_id(),
      SymbolInfo {
        export_kind: ExportKind::Class,
        is_reassigned: false,
      },
    );
  }
}

impl Visit for ExportScannerVisitor<'_> {
  fn visit_module_decl(&mut self, node: &ModuleDecl) {
    match node {
      ModuleDecl::ExportDecl(decl) => {
        if let Decl::Var(var) = &decl.decl {
          self.find_exports_from_var_decl(var);
        }
        if let Decl::Fn(func) = &decl.decl {
          self.find_exports_from_function_decl(func);
        }
        if let Decl::Class(class) = &decl.decl {
          self.find_exports_from_class_decl(class);
        }
      }
      ModuleDecl::ExportDefaultExpr(default_expr) => {
        self.find_exports_from_expr(&default_expr.expr);
      }
      ModuleDecl::ExportNamed(named_export) => {
        for specifier in &named_export.specifiers {
          self.find_exports_from_specifier(specifier);
        }
      }
      // These are not yet implemented but can be handled in the future.
      ModuleDecl::ExportDefaultDecl(_) => {}
      ModuleDecl::ExportAll(_) => {}
      // Types are skipped
      ModuleDecl::TsExportAssignment(_) => {}
      ModuleDecl::TsNamespaceExport(_) => {}
      // We ignore imports
      ModuleDecl::TsImportEquals(_) => {}
      ModuleDecl::Import(_) => {}
    }

    node.visit_children_with(self);
  }

  fn visit_assign_expr(&mut self, node: &AssignExpr) {
    if let AssignTarget::Simple(SimpleAssignTarget::Ident(ident)) = &node.left {
      self.reassignments.insert(ident.to_id());
    }

    node.visit_children_with(self);
  }
}

pub struct BindingVisitor<'a> {
  exported_identifiers: &'a mut HashMap<Id, HashSet<Id>>,
  reassignments: &'a mut HashSet<Id>,
  symbol_info: &'a mut HashMap<Id, SymbolInfo>,
}

impl<'a> BindingVisitor<'a> {
  fn new(
    symbol_info: &'a mut HashMap<Id, SymbolInfo>,
    exported_identifiers: &'a mut HashMap<Id, HashSet<Id>>,
    reassignments: &'a mut HashSet<Id>,
  ) -> Self {
    Self {
      symbol_info,
      exported_identifiers,
      reassignments,
    }
  }
}

impl Visit for BindingVisitor<'_> {
  /// When an export of a variable is found, we need to identify the kind of export
  ///
  /// For example:
  ///
  ///     export { x };
  ///
  /// Gives us an identifier `x` that we can use to identify the kind of export.
  ///
  ///     const x = 1;
  fn visit_var_decl(&mut self, node: &VarDecl) {
    let export_kind = export_kind_from_decl(node);
    for decl in &node.decls {
      if let Pat::Ident(binding_ident) = &decl.name {
        let id = binding_ident.id.to_id();
        if self.exported_identifiers.contains_key(&id) {
          self
            .exported_identifiers
            .get(&id)
            .unwrap()
            .iter()
            .for_each(|exported_ident| {
              self.symbol_info.insert(
                exported_ident.to_id(),
                SymbolInfo {
                  export_kind,
                  is_reassigned: self.reassignments.contains(&id),
                },
              );
            });
        }
      }
    }
  }

  fn visit_fn_decl(&mut self, node: &FnDecl) {
    let ident = node.ident.to_id();
    if self.exported_identifiers.contains_key(&ident) {
      self
        .exported_identifiers
        .get(&ident)
        .unwrap()
        .iter()
        .for_each(|exported_ident| {
          self.symbol_info.insert(
            exported_ident.to_id(),
            SymbolInfo {
              export_kind: ExportKind::Function,
              is_reassigned: self.reassignments.contains(&ident),
            },
          );
        });
    }
  }
}

pub struct EsmExportClassifier {
  pub symbol_info: HashMap<Id, SymbolInfo>,
  pub exports_rebinding_optimisation: bool,
}

impl EsmExportClassifier {
  pub fn new(exports_rebinding_optimisation: bool) -> Self {
    Self {
      symbol_info: HashMap::new(),
      exports_rebinding_optimisation,
    }
  }
}

impl Visit for EsmExportClassifier {
  fn visit_module(&mut self, module: &Module) {
    if !self.exports_rebinding_optimisation {
      // Skip all work when flag is off
      // This will mean we default to the original behaviour when the state of the symbol_info is checked
      return;
    }

    // First we scan for all esm exports
    let mut export_scanner_visitor = ExportScannerVisitor::new(&mut self.symbol_info);
    module.visit_with(&mut export_scanner_visitor);

    // We then want to look for any variable declarations that we've discovered references to
    let mut exported_identifiers = export_scanner_visitor.exported_identifiers;
    let mut reassignments = export_scanner_visitor.reassignments;
    let mut binding_visitor = BindingVisitor::new(
      &mut self.symbol_info,
      &mut exported_identifiers,
      &mut reassignments,
    );

    module.visit_with(&mut binding_visitor);

    // Finally, we may have already processed some export declarations but not discovered reassignments
    // so we need to update the symbol info for any reassignments detected from exports in the first pass (export_scanner_visitor)
    for (ident, symbol_info) in &mut self.symbol_info {
      if reassignments.contains(ident) {
        symbol_info.is_reassigned = true;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use atlaspack_swc_runner::test_utils::{run_test_visit_const, RunVisitResult};
  use swc_core::atoms::Atom;

  use crate::esm_export_classifier::{EsmExportClassifier, ExportKind, SymbolInfo};

  #[test]
  fn marks_exports_from_binding_ident() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const x = 'foo';
        export let y = 'bar';
        y = 'bar2';
        export var z = 'baz';
        z = 'baz2';
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("x")).unwrap(),
        symbol_info.get(&Atom::from("y")).unwrap(),
        symbol_info.get(&Atom::from("z")).unwrap(),
      ),
      (
        &&SymbolInfo {
          export_kind: ExportKind::Const,
          is_reassigned: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Let,
          is_reassigned: true,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Var,
          is_reassigned: true,
        },
      )
    );
  }

  #[test]
  fn marks_exports_with_object_rename() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const { main: foo } = obj;
        export const x = foo;
        export let { main: bar } = obj;
        export let y = foo;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("foo")).unwrap().export_kind,
        symbol_info.get(&Atom::from("x")).unwrap().export_kind,
        symbol_info.get(&Atom::from("bar")).unwrap().export_kind,
        symbol_info.get(&Atom::from("y")).unwrap().export_kind,
      ),
      (
        ExportKind::Const,
        ExportKind::Const,
        ExportKind::Let,
        ExportKind::Let
      )
    );
  }

  #[test]
  fn marks_exports_with_array_destructuring() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const { main: [foo] } = obj;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      symbol_info.get(&Atom::from("foo")).unwrap().export_kind,
      ExportKind::Const
    );
  }

  #[test]
  fn marks_exports_with_nested_destructuring() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        const obj = {
          main: {
            d: 'd',
          },
        };

        export const { main: { d } } = obj;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      symbol_info.get(&Atom::from("d")).unwrap().export_kind,
      ExportKind::Const
    );
  }

  #[test]
  fn marks_exports_with_rest_patterns() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const { main: foo, ...rest } = obj;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("foo")).unwrap().export_kind,
        symbol_info.get(&Atom::from("rest")).unwrap().export_kind,
      ),
      (ExportKind::Const, ExportKind::Const)
    );
  }

  #[test]
  fn marks_exports_with_assign_patterns() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const { main: foo = 1 } = obj;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      symbol_info.get(&Atom::from("foo")).unwrap().export_kind,
      ExportKind::Const
    );
  }

  #[test]
  fn marks_const_exports_with_default_binding() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        const x = 'x';
        export default x;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      symbol_info.get(&Atom::from("x")).unwrap().export_kind,
      ExportKind::Const
    );
  }

  #[test]
  fn marks_let_exports_with_default_binding() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        let x = 'x';
        x = 'x2';
        export default x;
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      symbol_info.get(&Atom::from("x")).unwrap(),
      &&SymbolInfo {
        export_kind: ExportKind::Let,
        is_reassigned: true,
      }
    );
  }

  #[test]
  fn marks_named_exports() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        const foo = 'foo';
        let bar = 'bar';
        bar = 'bar2';

        export { foo, bar as baz };
        export { foo as foo2 };
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("foo")).unwrap(),
        symbol_info.get(&Atom::from("baz")).unwrap(),
        symbol_info.get(&Atom::from("foo2")).unwrap(),
      ),
      (
        &&SymbolInfo {
          export_kind: ExportKind::Const,
          is_reassigned: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Let,
          is_reassigned: true,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Const,
          is_reassigned: false,
        }
      )
    );
  }

  #[test]
  fn marks_functions() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export function foo() {}
        function bar() {}
        export { bar as baz };
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("foo")).unwrap().export_kind,
        symbol_info.get(&Atom::from("baz")).unwrap().export_kind,
      ),
      (ExportKind::Function, ExportKind::Function)
    );
  }

  #[test]
  fn marks_functions_and_tracks_reassignments() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export function foo() {
          bar = 'bar';
        }
        function bar() {}
        export { bar as baz };
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("foo")).unwrap(),
        symbol_info.get(&Atom::from("baz")).unwrap(),
      ),
      (
        &&SymbolInfo {
          export_kind: ExportKind::Function,
          is_reassigned: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Function,
          is_reassigned: true,
        }
      )
    );
  }

  #[test]
  fn marks_functions_and_tracks_reassignments_in_nested_assignments() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export function foo() {
          const some_value = (bar = 'bar2');
        }
        function bar() {}
        export { bar as baz };
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (
        symbol_info.get(&Atom::from("foo")).unwrap(),
        symbol_info.get(&Atom::from("baz")).unwrap(),
      ),
      (
        &&SymbolInfo {
          export_kind: ExportKind::Function,
          is_reassigned: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Function,
          is_reassigned: true,
        }
      )
    );
  }

  #[test]
  fn marks_classes_and_tracks_reassignments() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export class Foo {}
        Foo = 'Foo';
      "#,
      |_context| EsmExportClassifier::new(true),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (symbol_info.get(&Atom::from("Foo")).unwrap()),
      (&&SymbolInfo {
        export_kind: ExportKind::Class,
        is_reassigned: true,
      })
    );
  }
}
