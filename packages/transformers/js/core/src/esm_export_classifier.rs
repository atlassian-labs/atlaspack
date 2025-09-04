use std::collections::{HashMap, HashSet};

use swc_core::common::Mark;
use swc_core::ecma::ast::*;
use swc_core::ecma::utils::ident::IdentLike;
use swc_core::ecma::visit::{Visit, VisitWith};

use crate::utils::match_member_expr;

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
  pub is_cjs_module: bool,
  pub has_export_all: bool,
}

impl SymbolInfo {
  pub fn new(export_kind: ExportKind, is_reassigned: bool) -> Self {
    Self {
      export_kind,
      is_reassigned,
      is_cjs_module: false,
      has_export_all: false,
    }
  }
}

pub struct SymbolsInfo {
  pub id_to_symbol_info: HashMap<Id, SymbolInfo>,
}

impl SymbolsInfo {
  fn new() -> Self {
    Self {
      id_to_symbol_info: HashMap::new(),
    }
  }

  pub fn is_static_binding_safe(&self, id: &Id) -> bool {
    match self.id_to_symbol_info.get(id) {
      Some(info) => {
        if info.is_cjs_module || info.has_export_all {
          // CJS modules can access the exports object so we need to bail out
          return false;
        }

        match info.export_kind {
          ExportKind::Const => true,
          // If the symbol is reassigned, we need to allow rebinding
          _ => !info.is_reassigned,
        }
      }
      // If the symbol is not found, we default to safe and allow rebinding
      // This is also the behaviour when the feature flag is disabled
      None => false,
    }
  }
}

impl Default for SymbolsInfo {
  fn default() -> Self {
    Self::new()
  }
}

pub struct ExportScannerVisitor<'a> {
  // Keep track of exported idenifiers (values are the export names)
  exported_identifiers: HashMap<Id, HashSet<Id>>,
  reassignments: HashSet<Id>,
  symbols_info: &'a mut SymbolsInfo,
  unresolved_mark: Mark,
  is_cjs_module: bool,
  has_export_all: bool,
}

fn export_kind_from_decl(decl: &VarDecl) -> ExportKind {
  match decl.kind {
    VarDeclKind::Const => ExportKind::Const,
    VarDeclKind::Let => ExportKind::Let,
    VarDeclKind::Var => ExportKind::Var,
  }
}

impl<'a> ExportScannerVisitor<'a> {
  fn new(symbols_info: &'a mut SymbolsInfo, unresolved_mark: Mark) -> ExportScannerVisitor<'a> {
    Self {
      exported_identifiers: HashMap::new(),
      reassignments: HashSet::new(),
      symbols_info,
      unresolved_mark,
      is_cjs_module: false,
      has_export_all: false,
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

  /// Finds exports in a binding ident.
  ///
  /// For example:
  ///
  /// ```javascript
  /// export const foo = 1;
  /// ```
  ///
  fn find_exports_from_binding_ident(
    &mut self,
    binding_ident: &BindingIdent,
    export_kind: &ExportKind,
  ) {
    let ident = binding_ident.id.clone();
    self
      .symbols_info
      .id_to_symbol_info
      .insert(ident.to_id(), SymbolInfo::new(*export_kind, false));
  }

  /// Finds exports from an object pattern.
  ///
  /// For example:
  ///
  /// ```javascript
  /// export const { foo } = obj;
  /// ```
  ///
  fn find_exports_from_object_pattern(&mut self, object_pat: &ObjectPat, export_kind: &ExportKind) {
    for prop in &object_pat.props {
      match prop {
        // This is `foo` in:
        // { foo }
        ObjectPatProp::Assign(prop) => {
          let key = prop.key.clone();
          assert!(prop.value.is_none());
          self
            .symbols_info
            .id_to_symbol_info
            .insert(key.to_id(), SymbolInfo::new(*export_kind, false));
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

  /// Finds exports from a pattern.
  ///
  /// This happens when we have:
  ///
  /// ```javascript
  /// export const { foo: <PAT> } = ...;
  /// ```
  ///
  /// There are many valid patterns, for example:
  ///
  /// ```javascript
  /// export const { foo: { bar } } = ...;
  /// ```
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

  /// Finds exports from a pattern.
  ///
  /// This happens when we have:
  ///
  /// ```javascript
  /// export const [<PAT>]= ...;
  /// ```
  ///
  /// There are many valid patterns, for example:
  ///
  /// ```javascript
  /// export const [foo, bar, ...rest, { abc }] = ...;
  /// ```
  ///
  /// We recursively extract exports from each element. Missing elements are ignored.
  /// An element is missing on this sample:
  ///
  /// ```javascript
  /// export const [one, two, , four] = ...;
  /// ```
  ///
  fn find_exports_from_array_pattern(&mut self, array_pat: &ArrayPat, export_kind: &ExportKind) {
    for elem in array_pat.elems.iter().flatten() {
      self.find_exports_from_pat(elem, export_kind);
    }
  }

  /// Finds exports from a rest pattern.
  ///
  /// This happens when we have:
  ///
  /// ```javascript
  /// export const { ...<PAT> } = ...;
  /// export const [ ...<PAT> ] = ...;
  /// ```
  ///
  /// We recursively extract exports from the `<PAT>` node.
  fn find_exports_from_rest_pattern(&mut self, rest_pat: &RestPat, export_kind: &ExportKind) {
    self.find_exports_from_pat(&rest_pat.arg, export_kind);
  }

  /// Finds exports from an assign pattern.
  ///
  /// This happens when we have:
  ///
  /// ```javascript
  /// export const { a = 10 } = ...;
  /// export const { <PAT> = <DEFAULT> } = ...;
  /// ```
  ///
  /// We recursively extract exports from `<PAT>`.
  fn find_exports_from_assign_pattern(&mut self, assign_pat: &AssignPat, export_kind: &ExportKind) {
    self.find_exports_from_pat(&assign_pat.left, export_kind);
  }

  /// Finds exports from an expression.
  ///
  /// This happens when we have an identifier:
  ///
  /// ```javascript
  /// export default foo;
  /// ```
  ///
  fn find_exports_from_expr(&mut self, expr: &Expr) {
    if let Expr::Ident(ident) = &expr {
      self
        .exported_identifiers
        .entry(ident.to_id())
        .or_default()
        .insert(ident.to_id());
    }
  }

  /// Finds exports from a specifier.
  ///
  /// This happens when we have a named export:
  ///
  /// ```javascript
  /// export { foo };
  /// ```
  ///
  /// Or a renamed export:
  ///
  /// ```javascript
  /// export { foo as bar };
  /// ```
  ///
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

  /// Finds exports from a function declaration.
  ///
  /// This happens when we have a function declaration:
  ///
  /// ```javascript
  /// export function foo() { ... }
  /// ```
  ///
  fn find_exports_from_function_decl(&mut self, func: &FnDecl) {
    self.symbols_info.id_to_symbol_info.insert(
      func.ident.to_id(),
      SymbolInfo::new(ExportKind::Function, false), // If we're here, this function was declared inside an export
    );
  }

  /// Finds exports from a class declaration.
  ///
  /// This happens when we have a class declaration:
  ///
  /// ```javascript
  /// export class Foo { ... }
  /// ```
  ///
  fn find_exports_from_class_decl(&mut self, class: &ClassDecl) {
    self.symbols_info.id_to_symbol_info.insert(
      class.ident.to_id(),
      SymbolInfo::new(ExportKind::Class, false),
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
      ModuleDecl::ExportAll(_) => {
        // It's complicated to determine if an export all is safe, so we default to safe and allow rebinding
        self.has_export_all = true;
      }
      // These are not yet implemented but can be handled in the future.
      ModuleDecl::ExportDefaultDecl(_) => {}
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

  fn visit_member_expr(&mut self, node: &MemberExpr) {
    if match_member_expr(node, vec!["module", "exports"], self.unresolved_mark) {
      self.is_cjs_module = true;
    }
    match (&*node.obj, &node.prop) {
      (Expr::Ident(obj), MemberProp::Ident(prop)) => {
        if obj.sym == "module" && prop.sym == "exports" {
          self.is_cjs_module = true;
        }
      }
      (Expr::Ident(obj), MemberProp::Computed(ComputedPropName { expr, .. })) => {
        if let Expr::Lit(Lit::Str(str)) = &**expr {
          if obj.sym == "module" && str.value == "exports" {
            self.is_cjs_module = true;
          }
        }
      }
      _ => {}
    }

    node.visit_children_with(self);
  }

  fn visit_ident(&mut self, node: &Ident) {
    if &*node.sym == "exports" {
      self.is_cjs_module = true;
    }
  }
}

pub struct BindingVisitor<'a> {
  exported_identifiers: &'a mut HashMap<Id, HashSet<Id>>,
  reassignments: &'a mut HashSet<Id>,
  symbols_info: &'a mut SymbolsInfo,
}

impl<'a> BindingVisitor<'a> {
  fn new(
    symbols_info: &'a mut SymbolsInfo,
    exported_identifiers: &'a mut HashMap<Id, HashSet<Id>>,
    reassignments: &'a mut HashSet<Id>,
  ) -> Self {
    Self {
      symbols_info,
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
  /// ```javascript
  /// export { x };
  /// ```
  ///
  /// Gives us an identifier `x` that we can use to identify the kind of export.
  ///
  /// ```javascript
  /// const x = 1;
  /// ```
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
              self.symbols_info.id_to_symbol_info.insert(
                exported_ident.to_id(),
                SymbolInfo::new(export_kind, self.reassignments.contains(&id)),
              );
            });
        }
      }
    }
  }

  /// If we find an exported identifier for a function, we need to track it
  ///
  /// For example:
  ///
  /// ```javascript
  /// function foo() { ... }
  ///
  /// export { foo };
  /// ```
  fn visit_fn_decl(&mut self, node: &FnDecl) {
    let ident = node.ident.to_id();

    if let Some(exported_idents) = self.exported_identifiers.get(&ident) {
      exported_idents.iter().for_each(|exported_ident| {
        self.symbols_info.id_to_symbol_info.insert(
          exported_ident.to_id(),
          SymbolInfo::new(ExportKind::Function, self.reassignments.contains(&ident)),
        );
      });
    }
  }

  /// If we find an exported identifier for a class, we need to track it
  ///
  /// For example:
  ///
  /// ```javascript
  /// class Foo { ... }
  ///
  /// export { Foo };
  /// ```
  fn visit_class_decl(&mut self, node: &ClassDecl) {
    let ident = node.ident.to_id();
    if let Some(exported_idents) = self.exported_identifiers.get(&ident) {
      exported_idents.iter().for_each(|exported_ident| {
        self.symbols_info.id_to_symbol_info.insert(
          exported_ident.to_id(),
          SymbolInfo::new(ExportKind::Class, self.reassignments.contains(&ident)),
        );
      });
    }
  }
}

pub struct EsmExportClassifier {
  pub symbols_info: SymbolsInfo,
  pub exports_rebinding_optimisation: bool,
  pub unresolved_mark: Mark,
}

impl EsmExportClassifier {
  pub fn new(exports_rebinding_optimisation: bool, unresolved_mark: Mark) -> Self {
    Self {
      symbols_info: SymbolsInfo::default(),
      exports_rebinding_optimisation,
      unresolved_mark,
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
    let (mut exported_identifiers, mut reassignments, is_cjs_module, has_export_all) = {
      let mut export_scanner_visitor =
        ExportScannerVisitor::new(&mut self.symbols_info, self.unresolved_mark);
      module.visit_with(&mut export_scanner_visitor);

      (
        export_scanner_visitor.exported_identifiers,
        export_scanner_visitor.reassignments,
        export_scanner_visitor.is_cjs_module,
        export_scanner_visitor.has_export_all,
      )
    };

    // We then want to look for any variable declarations that we've discovered references to
    let mut binding_visitor = BindingVisitor::new(
      &mut self.symbols_info,
      &mut exported_identifiers,
      &mut reassignments,
    );

    module.visit_with(&mut binding_visitor);

    for (ident, symbol_info) in &mut self.symbols_info.id_to_symbol_info {
      // Finally, we may have already processed some export declarations but not discovered reassignments
      // so we need to update the symbol info for any reassignments detected from exports in the first pass (export_scanner_visitor)
      if reassignments.contains(ident) {
        symbol_info.is_reassigned = true;
      }

      // We also want to know on each symbol if it's declared in a CJS module
      // This informs that we want to bail out
      if is_cjs_module {
        symbol_info.is_cjs_module = true;
      }

      if has_export_all {
        symbol_info.has_export_all = true;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use atlaspack_swc_runner::test_utils::{run_test_visit_const, RunVisitResult};
  use swc_core::{atoms::Atom, common::Mark};

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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
          is_cjs_module: false,
          has_export_all: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Let,
          is_reassigned: true,
          is_cjs_module: false,
          has_export_all: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Var,
          is_reassigned: true,
          is_cjs_module: false,
          has_export_all: false,
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      symbol_info.get(&Atom::from("x")).unwrap(),
      &&SymbolInfo {
        export_kind: ExportKind::Let,
        is_reassigned: true,
        is_cjs_module: false,
        has_export_all: false,
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
          is_cjs_module: false,
          has_export_all: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Let,
          is_reassigned: true,
          is_cjs_module: false,
          has_export_all: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Const,
          is_reassigned: false,
          is_cjs_module: false,
          has_export_all: false,
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
          is_cjs_module: false,
          has_export_all: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Function,
          is_reassigned: true,
          is_cjs_module: false,
          has_export_all: false,
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
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
          is_cjs_module: false,
          has_export_all: false,
        },
        &&SymbolInfo {
          export_kind: ExportKind::Function,
          is_reassigned: true,
          is_cjs_module: false,
          has_export_all: false,
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
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (symbol_info.get(&Atom::from("Foo")).unwrap()),
      (&&SymbolInfo {
        export_kind: ExportKind::Class,
        is_reassigned: true,
        is_cjs_module: false,
        has_export_all: false,
      })
    );
  }

  #[test]
  fn marks_classes_and_tracks_reassignments_when_exported_after() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        class Foo {}
        Foo = 'Foo';

        export { Foo };
      "#,
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (symbol_info.get(&Atom::from("Foo")).unwrap()),
      (&&SymbolInfo {
        export_kind: ExportKind::Class,
        is_reassigned: true,
        is_cjs_module: false,
        has_export_all: false,
      })
    );
  }

  #[test]
  fn marks_cjs_module() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const foo = 'foo';

        module.exports.bar = 'bar';
      "#,
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (symbol_info.get(&Atom::from("foo")).unwrap()),
      (&&SymbolInfo {
        export_kind: ExportKind::Const,
        is_reassigned: false,
        is_cjs_module: true,
        has_export_all: false,
      })
    );
  }

  #[test]
  fn marks_cjs_module_with_computed_property() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export const foo = 'foo';

        module["exports"]["bar"] = 'bar';
      "#,
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (symbol_info.get(&Atom::from("foo")).unwrap()),
      (&&SymbolInfo {
        export_kind: ExportKind::Const,
        is_reassigned: false,
        is_cjs_module: true,
        has_export_all: false,
      })
    );
  }

  #[test]
  fn marks_export_all() {
    let RunVisitResult { visitor, .. } = run_test_visit_const(
      r#"
        export * from 'foo';

        export const bar = 'bar';
      "#,
      |_context| EsmExportClassifier::new(true, Mark::fresh(Mark::root())),
    );

    let symbol_info: HashMap<Atom, &SymbolInfo> = visitor
      .symbols_info
      .id_to_symbol_info
      .iter()
      .map(|(key, value)| (key.0.clone(), value))
      .collect::<HashMap<_, _>>();

    assert_eq!(
      (symbol_info.get(&Atom::from("bar")).unwrap()),
      (&&SymbolInfo {
        export_kind: ExportKind::Const,
        is_reassigned: false,
        is_cjs_module: false,
        has_export_all: true,
      })
    );
  }
}
