use std::collections::HashMap;

use swc_core::ecma::ast::*;
use swc_core::ecma::visit::Visit;

#[derive(Debug, PartialEq, Clone)]
pub enum ExportKind {
  Const,
  Let,
  Var,
  // Function,
  // Class,
  // Literal,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SymbolInfo {
  pub export_kind: ExportKind,
}

pub struct EsmExportClassifier {
  pub symbol_info: HashMap<Id, SymbolInfo>,
}

impl EsmExportClassifier {
  fn export_kind_from_decl(&self, decl: &VarDecl) -> ExportKind {
    match decl.kind {
      VarDeclKind::Const => ExportKind::Const,
      VarDeclKind::Let => ExportKind::Let,
      VarDeclKind::Var => ExportKind::Var,
    }
  }

  fn find_exports_from_decl(&mut self, var: &VarDecl) {
    let export_kind = self.export_kind_from_decl(var);
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
        export_kind: export_kind.clone(),
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
              export_kind: export_kind.clone(),
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

  pub(crate) fn new() -> Self {
    Self {
      symbol_info: HashMap::new(),
    }
  }
}

impl Visit for EsmExportClassifier {
  fn visit_module(&mut self, node: &Module) {
    for item in &node.body {
      if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(decl)) = item {
        if let Decl::Var(var) = &decl.decl {
          self.find_exports_from_decl(var);
        }
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
        export var z = 'baz';
      "#,
      |_context| EsmExportClassifier::new(),
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
    assert_eq!(
      symbol_info.get(&Atom::from("y")).unwrap().export_kind,
      ExportKind::Let
    );
    assert_eq!(
      symbol_info.get(&Atom::from("z")).unwrap().export_kind,
      ExportKind::Var
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
      |_context| EsmExportClassifier::new(),
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
    assert_eq!(
      symbol_info.get(&Atom::from("x")).unwrap().export_kind,
      ExportKind::Const
    );
    assert_eq!(
      symbol_info.get(&Atom::from("bar")).unwrap().export_kind,
      ExportKind::Let
    );
    assert_eq!(
      symbol_info.get(&Atom::from("y")).unwrap().export_kind,
      ExportKind::Let
    );
  }

  #[test]
  fn marks_exports_with_array_destructuring() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const { main: [foo] } = obj;
      "#,
      |_context| EsmExportClassifier::new(),
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
      |_context| EsmExportClassifier::new(),
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
      |_context| EsmExportClassifier::new(),
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
    assert_eq!(
      symbol_info.get(&Atom::from("rest")).unwrap().export_kind,
      ExportKind::Const
    );
  }

  #[test]
  fn marks_exports_with_assign_patterns() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const { main: foo = 1 } = obj;
      "#,
      |_context| EsmExportClassifier::new(),
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
}
