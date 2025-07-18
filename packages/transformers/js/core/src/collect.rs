use std::collections::HashMap;
use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;
use swc_core::common::sync::Lrc;
use swc_core::common::Mark;
use swc_core::common::Span;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::js_word;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::utils::stack_size::maybe_grow_default;
use swc_core::ecma::visit::noop_visit_type;
use swc_core::ecma::visit::Visit;
use swc_core::ecma::visit::VisitWith;

use crate::id;
use crate::utils::is_unresolved;
use crate::utils::match_export_name;
use crate::utils::match_export_name_ident;
use crate::utils::match_import;
use crate::utils::match_import_cond;
use crate::utils::match_member_expr;
use crate::utils::match_property_name;
use crate::utils::match_require;
use crate::utils::Bailout;
use crate::utils::BailoutReason;
use crate::utils::SourceLocation;

macro_rules! collect_visit_fn {
  ($name:ident, $type:ident) => {
    fn $name(&mut self, node: &$type) {
      let in_module_this = self.in_module_this;
      let in_function = self.in_function;
      self.in_module_this = false;
      self.in_function = true;
      node.visit_children_with(self);
      self.in_module_this = in_module_this;
      self.in_function = in_function;
    }
  };
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Serialize)]
pub enum ImportKind {
  Require,
  Import,
  DynamicImport,
  ConditionalImport,
}

#[derive(Debug)]
pub struct Import {
  pub source: JsWord,
  pub specifier: JsWord,
  pub kind: ImportKind,
  pub loc: SourceLocation,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Export {
  pub source: Option<JsWord>,
  pub specifier: JsWord,
  pub loc: SourceLocation,
  pub is_esm: bool,
}

pub struct Collect {
  pub source_map: Lrc<swc_core::common::SourceMap>,
  pub unresolved_mark: Mark,
  pub ignore_mark: Mark,
  pub global_mark: Mark,
  pub static_cjs_exports: bool,
  pub has_cjs_exports: bool,
  pub is_esm: bool,
  pub should_wrap: bool,
  /// local variable binding -> descriptor
  pub imports: HashMap<Id, Import>,
  pub this_exprs: HashMap<JsWord, Span>,
  /// exported name -> descriptor
  pub exports: HashMap<JsWord, Export>,
  /// local variable binding -> exported name
  pub exports_locals: HashMap<Id, JsWord>,
  /// source of the export-all --> location
  pub exports_all: HashMap<JsWord, SourceLocation>,
  /// the keys in `imports` that are actually used (referenced), except namespace imports
  pub used_imports: HashSet<Id>,
  pub non_static_access: HashMap<Id, Vec<Span>>,
  pub non_const_bindings: HashMap<Id, Vec<Span>>,
  pub non_static_requires: HashSet<JsWord>,
  pub wrapped_requires: HashSet<String>,
  pub bailouts: Option<Vec<Bailout>>,
  pub is_empty_or_empty_export: bool,
  pub computed_properties_fix: bool,
  in_module_this: bool,
  in_top_level: bool,
  in_export_decl: bool,
  in_function: bool,
  in_assign: bool,
  in_class: bool,
  is_module: bool,
  conditional_bundling: bool,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CollectImportedSymbol {
  pub source: JsWord,
  pub local: JsWord,
  pub imported: JsWord,
  pub loc: SourceLocation,
  pub kind: ImportKind,
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct CollectExportedSymbol {
  pub source: Option<JsWord>,
  pub local: JsWord,
  pub exported: JsWord,
  pub loc: SourceLocation,
}

#[derive(Debug, Serialize)]
pub struct CollectExportedAll {
  pub source: JsWord,
  pub loc: SourceLocation,
}

#[derive(Serialize, Debug)]
#[non_exhaustive]
pub struct CollectResult {
  pub imports: Vec<CollectImportedSymbol>,
  pub exports: Vec<CollectExportedSymbol>,
  pub exports_all: Vec<CollectExportedAll>,
  pub should_wrap: bool,
  pub has_cjs_exports: bool,
  pub is_esm: bool,
  pub is_empty: bool,
}

impl Collect {
  // An extra argument has been added here for the computed_properties_fix, but it bumps the
  // function up over the arguments limit. Rather than reworking how feature flags work, just
  // setting this to ignore the lint warning for now.
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    source_map: Lrc<swc_core::common::SourceMap>,
    unresolved_mark: Mark,
    ignore_mark: Mark,
    global_mark: Mark,
    trace_bailouts: bool,
    is_module: bool,
    conditional_bundling: bool,
    computed_properties_fix: bool,
  ) -> Self {
    Collect {
      source_map,
      unresolved_mark,
      ignore_mark,
      global_mark,
      is_module,
      static_cjs_exports: true,
      has_cjs_exports: false,
      is_esm: false,
      should_wrap: false,
      imports: HashMap::new(),
      this_exprs: HashMap::new(),
      exports: HashMap::new(),
      exports_locals: HashMap::new(),
      exports_all: HashMap::new(),
      used_imports: HashSet::new(),
      non_static_access: HashMap::new(),
      non_const_bindings: HashMap::new(),
      non_static_requires: HashSet::new(),
      wrapped_requires: HashSet::new(),
      in_module_this: true,
      in_top_level: true,
      in_export_decl: false,
      in_function: false,
      in_assign: false,
      in_class: false,
      bailouts: if trace_bailouts { Some(vec![]) } else { None },
      conditional_bundling,
      is_empty_or_empty_export: false,
      computed_properties_fix,
    }
  }
}

impl From<Collect> for CollectResult {
  fn from(collect: Collect) -> CollectResult {
    let imports = collect
      .imports
      .into_iter()
      .filter(|(local, _)| collect.used_imports.contains(local))
      .map(
        |(
          local,
          Import {
            source,
            specifier,
            loc,
            kind,
          },
        )| CollectImportedSymbol {
          source,
          local: local.0,
          imported: specifier,
          loc,
          kind,
        },
      )
      .collect();

    let mut exports: Vec<CollectExportedSymbol> = collect
      .exports
      .into_iter()
      .map(
        |(
          exported,
          Export {
            source,
            specifier,
            loc,
            ..
          },
        )| CollectExportedSymbol {
          source,
          local: specifier,
          exported,
          loc,
        },
      )
      .collect();

    // Add * symbol if there are any CJS exports so that unknown symbols don't cause errors (e.g. default interop).
    if collect.has_cjs_exports {
      exports.push(CollectExportedSymbol {
        source: None,
        exported: "*".into(),
        local: "_".into(),
        loc: SourceLocation {
          start_line: 1,
          start_col: 1,
          end_line: 1,
          end_col: 1,
        },
      })
    }

    CollectResult {
      imports,
      exports,
      exports_all: collect
        .exports_all
        .into_iter()
        .map(|(source, loc)| CollectExportedAll { source, loc })
        .collect(),
      should_wrap: collect.should_wrap,
      has_cjs_exports: collect.has_cjs_exports,
      is_esm: collect.is_esm,
      is_empty: collect.is_empty_or_empty_export,
    }
  }
}

impl Visit for Collect {
  fn visit_module(&mut self, node: &Module) {
    self.in_module_this = true;
    self.in_top_level = true;
    self.in_function = false;

    if node.body.is_empty() {
      self.is_empty_or_empty_export = true;
    }
    if node.body.len() == 1 {
      // Check if the single module item is an empty export
      if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) = &node.body[0] {
        self.is_empty_or_empty_export =
          named_export.specifiers.is_empty() && named_export.src.is_none();
      }
    }

    // Visit all imports first so that all imports are known when collecting used_imports
    for n in &node.body {
      if n.is_module_decl() {
        n.visit_with(self);
      }
    }
    for n in &node.body {
      if !n.is_module_decl() {
        n.visit_with(self);
      }
    }
    self.in_module_this = false;

    for (key, span) in std::mem::take(&mut self.this_exprs) {
      if self.exports.contains_key(&key) {
        self.should_wrap = true;
        self.add_bailout(span, BailoutReason::ThisInExport);
      }
    }

    if let Some(bailouts) = &mut self.bailouts {
      for (key, Import { specifier, .. }) in &self.imports {
        if specifier == "*" {
          if let Some(spans) = self.non_static_access.get(key) {
            for span in spans {
              bailouts.push(Bailout {
                loc: SourceLocation::from(&self.source_map, *span),
                reason: BailoutReason::NonStaticAccess,
              })
            }
          }
        }
      }

      bailouts.sort_by(|a, b| a.loc.partial_cmp(&b.loc).unwrap());
    }
  }

  collect_visit_fn!(visit_function, Function);
  collect_visit_fn!(visit_getter_prop, GetterProp);
  collect_visit_fn!(visit_setter_prop, SetterProp);

  fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
    let in_function = self.in_function;
    self.in_function = true;
    node.visit_children_with(self);
    self.in_function = in_function;
  }

  fn visit_module_item(&mut self, node: &ModuleItem) {
    match node {
      ModuleItem::ModuleDecl(_decl) => {
        self.is_esm = true;
      }
      ModuleItem::Stmt(stmt) => {
        match stmt {
          Stmt::Decl(decl) => {
            if let Decl::Var(_var) = decl {
              decl.visit_children_with(self);
              return;
            }
          }
          Stmt::Expr(expr) => {
            // Top-level require(). Do not traverse further so it is not marked as wrapped.
            if let Some(_source) = self.match_require(&expr.expr) {
              return;
            }

            // TODO: optimize `require('foo').bar` / `require('foo').bar()` as well
          }
          _ => {}
        }
      }
    }

    self.in_top_level = false;
    node.visit_children_with(self);
    self.in_top_level = true;
  }

  fn visit_import_decl(&mut self, node: &ImportDecl) {
    for specifier in &node.specifiers {
      match specifier {
        ImportSpecifier::Named(named) => {
          let imported = match &named.imported {
            Some(imported) => match_export_name(imported).0.clone(),
            None => named.local.sym.clone(),
          };
          self.imports.insert(
            id!(named.local),
            Import {
              source: node.src.value.clone(),
              specifier: imported,
              kind: ImportKind::Import,
              loc: SourceLocation::from(&self.source_map, named.span),
            },
          );
        }
        ImportSpecifier::Default(default) => {
          self.imports.insert(
            id!(default.local),
            Import {
              source: node.src.value.clone(),
              specifier: js_word!("default"),
              kind: ImportKind::Import,
              loc: SourceLocation::from(&self.source_map, default.span),
            },
          );
        }
        ImportSpecifier::Namespace(namespace) => {
          self.imports.insert(
            id!(namespace.local),
            Import {
              source: node.src.value.clone(),
              specifier: "*".into(),
              kind: ImportKind::Import,
              loc: SourceLocation::from(&self.source_map, namespace.span),
            },
          );
        }
      }
    }
  }

  fn visit_named_export(&mut self, node: &NamedExport) {
    for specifier in &node.specifiers {
      let source = node.src.as_ref().map(|s| s.value.clone());
      match specifier {
        ExportSpecifier::Named(named) => {
          let exported = match &named.exported {
            Some(exported) => match_export_name(exported),
            None => match_export_name(&named.orig),
          };
          let orig = match_export_name_ident(&named.orig);
          let is_reexport = if source.is_none() {
            // import {foo} from "xyz";
            // export {foo};
            self.imports.get(&id!(orig))
          } else {
            None
          };
          let (source, specifier) = if let Some(reexport) = is_reexport {
            (Some(reexport.source.clone()), reexport.specifier.clone())
          } else {
            (source, orig.sym.clone())
          };
          self.exports.insert(
            exported.0.clone(),
            Export {
              specifier,
              loc: SourceLocation::from(&self.source_map, exported.1),
              source,
              is_esm: true,
            },
          );
          if node.src.is_none() {
            self
              .exports_locals
              .entry(id!(match_export_name_ident(&named.orig)))
              .or_insert_with(|| exported.0.clone());
          }
        }
        ExportSpecifier::Default(default) => {
          self.exports.insert(
            js_word!("default"),
            Export {
              specifier: default.exported.sym.clone(),
              loc: SourceLocation::from(&self.source_map, default.exported.span),
              source,
              is_esm: true,
            },
          );
          if node.src.is_none() {
            self
              .exports_locals
              .entry(id!(default.exported))
              .or_insert_with(|| js_word!("default"));
          }
        }
        ExportSpecifier::Namespace(namespace) => {
          self.exports.insert(
            match_export_name(&namespace.name).0,
            Export {
              specifier: "*".into(),
              loc: SourceLocation::from(&self.source_map, namespace.span),
              source,
              is_esm: true,
            },
          );
          // Populating exports_locals with * doesn't make any sense at all
          // and hoist doesn't use this anyway.
        }
      }
    }
  }

  fn visit_export_decl(&mut self, node: &ExportDecl) {
    match &node.decl {
      Decl::Class(class) => {
        self.exports.insert(
          class.ident.sym.clone(),
          Export {
            specifier: class.ident.sym.clone(),
            loc: SourceLocation::from(&self.source_map, class.ident.span),
            source: None,
            is_esm: true,
          },
        );
        self
          .exports_locals
          .entry(id!(class.ident))
          .or_insert_with(|| class.ident.sym.clone());
      }
      Decl::Fn(func) => {
        self.exports.insert(
          func.ident.sym.clone(),
          Export {
            specifier: func.ident.sym.clone(),
            loc: SourceLocation::from(&self.source_map, func.ident.span),
            source: None,
            is_esm: true,
          },
        );
        self
          .exports_locals
          .entry(id!(func.ident))
          .or_insert_with(|| func.ident.sym.clone());
      }
      Decl::Var(var) => {
        for decl in &var.decls {
          self.in_export_decl = true;
          decl.name.visit_with(self);
          self.in_export_decl = false;

          decl.init.visit_with(self);
        }
      }
      _ => {}
    }

    node.visit_children_with(self);
  }

  fn visit_export_default_decl(&mut self, node: &ExportDefaultDecl) {
    match &node.decl {
      DefaultDecl::Class(class) => {
        if let Some(ident) = &class.ident {
          self.exports.insert(
            js_word!("default"),
            Export {
              specifier: ident.sym.clone(),
              loc: SourceLocation::from(&self.source_map, node.span),
              source: None,
              is_esm: true,
            },
          );
          self
            .exports_locals
            .entry(id!(ident))
            .or_insert_with(|| js_word!("default"));
        } else {
          self.exports.insert(
            js_word!("default"),
            Export {
              specifier: js_word!("default"),
              loc: SourceLocation::from(&self.source_map, node.span),
              source: None,
              is_esm: true,
            },
          );
        }
      }
      DefaultDecl::Fn(func) => {
        if let Some(ident) = &func.ident {
          self.exports.insert(
            js_word!("default"),
            Export {
              specifier: ident.sym.clone(),
              loc: SourceLocation::from(&self.source_map, node.span),
              source: None,
              is_esm: true,
            },
          );
          self
            .exports_locals
            .entry(id!(ident))
            .or_insert_with(|| js_word!("default"));
        } else {
          self.exports.insert(
            js_word!("default"),
            Export {
              specifier: js_word!("default"),
              loc: SourceLocation::from(&self.source_map, node.span),
              source: None,
              is_esm: true,
            },
          );
        }
      }
      _ => {
        unreachable!("unsupported export default declaration");
      }
    };

    node.visit_children_with(self);
  }

  fn visit_export_default_expr(&mut self, node: &ExportDefaultExpr) {
    self.exports.insert(
      js_word!("default"),
      Export {
        specifier: js_word!("default"),
        loc: SourceLocation::from(&self.source_map, node.span),
        source: None,
        is_esm: true,
      },
    );

    node.visit_children_with(self);
  }

  fn visit_export_all(&mut self, node: &ExportAll) {
    self.exports_all.insert(
      node.src.value.clone(),
      SourceLocation::from(&self.source_map, node.span),
    );
  }

  fn visit_return_stmt(&mut self, node: &ReturnStmt) {
    if !self.in_function {
      self.should_wrap = true;
      self.add_bailout(node.span, BailoutReason::TopLevelReturn);
    }

    node.visit_children_with(self)
  }

  fn visit_binding_ident(&mut self, node: &BindingIdent) {
    if self.in_export_decl {
      self.exports.insert(
        node.id.sym.clone(),
        Export {
          specifier: node.id.sym.clone(),
          loc: SourceLocation::from(&self.source_map, node.id.span),
          source: None,
          is_esm: true,
        },
      );
      self
        .exports_locals
        .entry(id!(node.id))
        .or_insert_with(|| node.id.sym.clone());
    }

    if self.in_assign && node.id.ctxt.has_mark(self.global_mark) {
      self
        .non_const_bindings
        .entry(id!(node.id))
        .or_default()
        .push(node.id.span);
    }
  }

  fn visit_assign_pat_prop(&mut self, node: &AssignPatProp) {
    if self.in_export_decl {
      self.exports.insert(
        node.key.sym.clone(),
        Export {
          specifier: node.key.sym.clone(),
          loc: SourceLocation::from(&self.source_map, node.key.span),
          source: None,
          is_esm: true,
        },
      );
      self
        .exports_locals
        .entry(id!(node.key))
        .or_insert_with(|| node.key.sym.clone());
    }

    if self.in_assign && node.key.ctxt.has_mark(self.global_mark) {
      self
        .non_const_bindings
        .entry(id!(node.key))
        .or_default()
        .push(node.key.span);
    }

    node.value.visit_with(self);
  }

  fn visit_member_expr(&mut self, node: &MemberExpr) {
    // if module.exports, ensure only assignment or static member expression
    // if exports, ensure only static member expression
    // if require, could be static access (handle in fold)

    if match_member_expr(node, vec!["module", "exports"], self.unresolved_mark) {
      self.static_cjs_exports = false;
      self.has_cjs_exports = true;
      return;
    }

    if match_member_expr(node, vec!["module", "hot"], self.unresolved_mark) {
      return;
    }

    if match_member_expr(node, vec!["module", "require"], self.unresolved_mark) {
      return;
    }

    macro_rules! handle_export {
      () => {
        self.has_cjs_exports = true;
        if let Some((name, span)) = match_property_name(&node) {
          self.exports.insert(
            name.clone(),
            Export {
              specifier: name,
              source: None,
              loc: SourceLocation::from(&self.source_map, span),
              is_esm: false,
            },
          );
        } else {
          self.static_cjs_exports = false;
          self.add_bailout(node.span, BailoutReason::NonStaticExports);
        }
      };
    }

    match &*node.obj {
      Expr::Member(member) => {
        if match_member_expr(member, vec!["module", "exports"], self.unresolved_mark) {
          handle_export!();
          return;
        } else {
          member.visit_with(self);
        }
      }
      Expr::Ident(ident) => {
        if &*ident.sym == "exports" && is_unresolved(ident, self.unresolved_mark) {
          handle_export!();
          return;
        }

        if ident.sym == js_word!("module") && is_unresolved(ident, self.unresolved_mark) {
          self.has_cjs_exports = true;
          self.static_cjs_exports = false;
          self.should_wrap = true;
          self.add_bailout(node.span, BailoutReason::FreeModule);
          return;
        }

        if !self.computed_properties_fix && match_property_name(node).is_none() {
          self
            .non_static_access
            .entry(id!(ident))
            .or_default()
            .push(node.span);

          return;
        }

        if self.imports.contains_key(&id!(ident)) {
          self.used_imports.insert(id!(ident));

          if self.computed_properties_fix && match_property_name(node).is_none() {
            self
              .non_static_access
              .entry(id!(ident))
              .or_default()
              .push(node.span);
          }
        }

        return;
      }
      Expr::This(_this) => {
        if self.in_module_this {
          if !self.is_module {
            handle_export!();
          }
        } else if !self.in_class {
          if let MemberProp::Ident(prop) = &node.prop {
            self.this_exprs.insert(prop.sym.clone(), node.span);
          }
        }
        return;
      }
      _ => {}
    }

    node.visit_children_with(self);
  }

  fn visit_unary_expr(&mut self, node: &UnaryExpr) {
    if node.op == UnaryOp::TypeOf {
      match &*node.arg {
        Expr::Ident(ident)
          if ident.sym == js_word!("module") && is_unresolved(ident, self.unresolved_mark) =>
        {
          // Do nothing to avoid the ident visitor from marking the module as non-static.
        }
        _ => node.visit_children_with(self),
      }
    } else {
      node.visit_children_with(self);
    }
  }

  fn visit_expr(&mut self, node: &Expr) {
    // If we reached this visitor, this is a non-top-level require that isn't in a variable
    // declaration. We need to wrap the referenced module to preserve side effect ordering.
    if let Some(source) = self.match_require(node) {
      self.wrapped_requires.insert(source.to_string());
      let span = match node {
        Expr::Call(c) => c.span,
        _ => unreachable!(),
      };
      self.add_bailout(span, BailoutReason::NonTopLevelRequire);
    }

    if let Some(source) = match_import(node) {
      self.non_static_requires.insert(source.clone());
      self.wrapped_requires.insert(source.to_string());
      let span = match node {
        Expr::Call(c) => c.span,
        _ => unreachable!(),
      };
      self.add_bailout(span, BailoutReason::NonStaticDynamicImport);
    }

    if self.conditional_bundling {
      if let Some((source_true, source_false)) = match_import_cond(node, self.ignore_mark) {
        self.wrapped_requires.insert(source_true.to_string());
        self.wrapped_requires.insert(source_false.to_string());
        let span = match node {
          Expr::Call(c) => c.span,
          _ => unreachable!(),
        };
        self.add_bailout(span, BailoutReason::NonStaticDynamicImport);
      }
    }

    match node {
      Expr::Ident(ident) => {
        // Bail if `module` or `exports` are accessed non-statically.
        let is_module = ident.sym == js_word!("module");
        let is_exports = &*ident.sym == "exports";
        if (is_module || is_exports) && is_unresolved(ident, self.unresolved_mark) {
          self.has_cjs_exports = true;
          self.static_cjs_exports = false;
          if is_module {
            self.should_wrap = true;
            self.add_bailout(ident.span, BailoutReason::FreeModule);
          } else {
            self.add_bailout(ident.span, BailoutReason::FreeExports);
          }
        }

        self
          .non_static_access
          .entry(id!(ident))
          .or_default()
          .push(ident.span);

        if self.imports.contains_key(&id!(ident)) {
          self.used_imports.insert(id!(ident));
        }
      }
      _ => {
        maybe_grow_default(|| node.visit_children_with(self));
      }
    }
  }

  fn visit_ident(&mut self, node: &Ident) {
    // This visitor helps us identify used imports in cases like:
    //
    //   import { foo } from "bar";
    //   const baz = { foo };
    if self.imports.contains_key(&id!(node)) {
      self.used_imports.insert(id!(node));
    }
  }

  fn visit_class(&mut self, class: &Class) {
    let in_module_this = self.in_module_this;
    let in_function = self.in_function;
    let in_class = self.in_class;

    self.in_module_this = false;
    self.in_function = true;
    self.in_class = true;

    class.visit_children_with(self);
    self.in_module_this = in_module_this;
    self.in_function = in_function;
    self.in_class = in_class;
  }

  fn visit_this_expr(&mut self, node: &ThisExpr) {
    if !self.is_module && self.in_module_this {
      self.has_cjs_exports = true;
      self.static_cjs_exports = false;
      self.add_bailout(node.span, BailoutReason::FreeExports);
    }
  }

  fn visit_assign_expr(&mut self, node: &AssignExpr) {
    // if rhs is a require, record static accesses
    // if lhs is `exports`, mark as CJS exports re-assigned
    // if lhs is `module.exports`
    // if lhs is `module.exports.XXX` or `exports.XXX`, record static export

    self.in_assign = true;
    node.left.visit_with(self);
    self.in_assign = false;
    node.right.visit_with(self);

    if has_binding_identifier(&node.left, &"exports".into(), self.unresolved_mark) {
      // Must wrap for cases like
      // ```
      // function logExports() {
      //   console.log(exports);
      // }
      // exports.test = 1;
      // logExports();
      // exports = {test: 4};
      // logExports();
      // ```
      self.static_cjs_exports = false;
      self.has_cjs_exports = true;
      self.should_wrap = true;
      self.add_bailout(node.span, BailoutReason::ExportsReassignment);
    } else if has_binding_identifier(&node.left, &"module".into(), self.unresolved_mark) {
      // Same for `module`. If it is reassigned we can't correctly statically analyze.
      self.static_cjs_exports = false;
      self.has_cjs_exports = true;
      self.should_wrap = true;
      self.add_bailout(node.span, BailoutReason::ModuleReassignment);
    }
  }

  fn visit_var_declarator(&mut self, node: &VarDeclarator) {
    // if init is a require call, record static accesses
    if let Some(init) = &node.init {
      if let Some(source) = self.match_require(init) {
        self.add_pat_imports(&node.name, &source, ImportKind::Require);
        return;
      }

      match &**init {
        Expr::Member(member) => {
          if let Some(source) = self.match_require(&member.obj) {
            // Convert member expression on require to a destructuring assignment.
            // const yx = require('y').x; -> const {x: yx} = require('x');
            let key = match &member.prop {
              MemberProp::Computed(_) => PropName::Computed(ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(*member.obj.clone()),
              }),
              MemberProp::Ident(ident) => PropName::Ident(ident.clone()),
              _ => unreachable!(),
            };

            self.add_pat_imports(
              &Pat::Object(ObjectPat {
                optional: false,
                span: DUMMY_SP,
                type_ann: None,
                props: vec![ObjectPatProp::KeyValue(KeyValuePatProp {
                  key,
                  value: Box::new(node.name.clone()),
                })],
              }),
              &source,
              ImportKind::Require,
            );
            return;
          }
        }
        Expr::Await(await_exp) => {
          // let x = await import('foo');
          // let {x} = await import('foo');
          if let Some(source) = match_import(&await_exp.arg) {
            self.add_pat_imports(&node.name, &source, ImportKind::DynamicImport);
            return;
          }
        }
        _ => {}
      }
    }

    // This is visited via visit_module_item with is_top_level == true, it needs to be
    // set to false for called visitors (and restored again).
    let in_top_level = self.in_top_level;
    self.in_top_level = false;
    node.visit_children_with(self);
    self.in_top_level = in_top_level;
  }

  fn visit_call_expr(&mut self, node: &CallExpr) {
    if let Callee::Expr(expr) = &node.callee {
      match &**expr {
        Expr::Ident(ident) => {
          if ident.sym == js_word!("eval") && is_unresolved(ident, self.unresolved_mark) {
            self.should_wrap = true;
            self.add_bailout(node.span, BailoutReason::Eval);
          }
        }
        Expr::Member(member) => {
          // import('foo').then(foo => ...);
          if let Some(source) = match_import(&member.obj) {
            if match_property_name(member).map_or(false, |f| &*f.0 == "then") {
              if let Some(ExprOrSpread { expr, .. }) = node.args.first() {
                let param = match &**expr {
                  Expr::Fn(func) => func.function.params.first().map(|param| &param.pat),
                  Expr::Arrow(arrow) => arrow.params.first(),
                  _ => None,
                };

                if let Some(param) = param {
                  self.add_pat_imports(param, &source, ImportKind::DynamicImport);
                } else {
                  self.non_static_requires.insert(source.clone());
                  self.wrapped_requires.insert(source.to_string());
                  self.add_bailout(node.span, BailoutReason::NonStaticDynamicImport);
                }

                expr.visit_with(self);
                return;
              }
            }
          }
        }
        _ => {}
      }
    }

    node.visit_children_with(self);
  }
}

impl Collect {
  pub fn match_require(&self, node: &Expr) -> Option<JsWord> {
    match_require(node, self.unresolved_mark, self.ignore_mark)
  }

  fn add_pat_imports(&mut self, node: &Pat, src: &JsWord, kind: ImportKind) {
    if !self.in_top_level {
      match kind {
        ImportKind::Import => self
          .wrapped_requires
          .insert(format!("{}{}", src.clone(), "esm")),
        ImportKind::DynamicImport | ImportKind::Require | ImportKind::ConditionalImport => {
          self.wrapped_requires.insert(src.to_string())
        }
      };
      if kind != ImportKind::DynamicImport && kind != ImportKind::ConditionalImport {
        self.non_static_requires.insert(src.clone());
        let span = match node {
          Pat::Ident(id) => id.id.span,
          Pat::Array(arr) => arr.span,
          Pat::Object(obj) => obj.span,
          Pat::Rest(rest) => rest.span,
          Pat::Assign(assign) => assign.span,
          Pat::Invalid(i) => i.span,
          Pat::Expr(_) => DUMMY_SP,
        };
        self.add_bailout(span, BailoutReason::NonTopLevelRequire);
      }
    }

    match node {
      Pat::Ident(ident) => {
        // let x = require('y');
        // Need to track member accesses of `x`.
        self.imports.insert(
          id!(ident.id),
          Import {
            source: src.clone(),
            specifier: "*".into(),
            kind,
            loc: SourceLocation::from(&self.source_map, ident.id.span),
          },
        );
      }
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              let imported = match &kv.key {
                PropName::Ident(ident) => ident.sym.clone(),
                PropName::Str(str) => str.value.clone(),
                _ => {
                  // Non-static. E.g. computed property.
                  self.non_static_requires.insert(src.clone());
                  self.add_bailout(object.span, BailoutReason::NonStaticDestructuring);
                  continue;
                }
              };

              match &*kv.value {
                Pat::Ident(ident) => {
                  // let {x: y} = require('y');
                  // Need to track `x` as a used symbol.
                  self.imports.insert(
                    id!(ident.id),
                    Import {
                      source: src.clone(),
                      specifier: imported,
                      kind,
                      loc: SourceLocation::from(&self.source_map, ident.id.span),
                    },
                  );

                  // Mark as non-constant. CJS exports can be mutated by other modules,
                  // so it's not safe to reference them directly.
                  self
                    .non_const_bindings
                    .entry(id!(ident.id))
                    .or_default()
                    .push(ident.id.span);
                }
                _ => {
                  // Non-static.
                  self.non_static_requires.insert(src.clone());
                  self.add_bailout(object.span, BailoutReason::NonStaticDestructuring);
                }
              }
            }
            ObjectPatProp::Assign(assign) => {
              // let {x} = require('y');
              // let {x = 2} = require('y');
              // Need to track `x` as a used symbol.
              self.imports.insert(
                id!(assign.key),
                Import {
                  source: src.clone(),
                  specifier: assign.key.sym.clone(),
                  kind,
                  loc: SourceLocation::from(&self.source_map, assign.key.span),
                },
              );
              self
                .non_const_bindings
                .entry(id!(assign.key))
                .or_default()
                .push(assign.key.span);
            }
            ObjectPatProp::Rest(_rest) => {
              // let {x, ...y} = require('y');
              // Non-static. We don't know what keys are used.
              self.non_static_requires.insert(src.clone());
              self.add_bailout(object.span, BailoutReason::NonStaticDestructuring);
            }
          }
        }
      }
      _ => {
        // Non-static.
        self.non_static_requires.insert(src.clone());
        let span = match node {
          Pat::Ident(id) => id.id.span,
          Pat::Array(arr) => arr.span,
          Pat::Object(obj) => obj.span,
          Pat::Rest(rest) => rest.span,
          Pat::Assign(assign) => assign.span,
          Pat::Invalid(i) => i.span,
          Pat::Expr(_) => DUMMY_SP,
        };
        self.add_bailout(span, BailoutReason::NonStaticDestructuring);
      }
    }
  }

  pub fn get_non_const_binding_idents(&self, node: &Pat, idents: &mut Vec<Ident>) {
    match node {
      Pat::Ident(ident) => {
        if self.non_const_bindings.contains_key(&id!(ident.id)) {
          idents.push(ident.id.clone());
        }
      }
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              self.get_non_const_binding_idents(&kv.value, idents);
            }
            ObjectPatProp::Assign(assign) => {
              if self.non_const_bindings.contains_key(&id!(assign.key)) {
                idents.push(assign.key.id.clone());
              }
            }
            ObjectPatProp::Rest(rest) => {
              self.get_non_const_binding_idents(&rest.arg, idents);
            }
          }
        }
      }
      Pat::Array(array) => {
        for el in array.elems.iter().flatten() {
          self.get_non_const_binding_idents(el, idents);
        }
      }
      _ => {}
    }
  }

  fn add_bailout(&mut self, span: Span, reason: BailoutReason) {
    if let Some(bailouts) = &mut self.bailouts {
      bailouts.push(Bailout {
        loc: SourceLocation::from(&self.source_map, span),
        reason,
      })
    }
  }
}

fn has_binding_identifier(node: &AssignTarget, sym: &JsWord, unresolved_mark: Mark) -> bool {
  pub struct BindingIdentFinder<'a> {
    sym: &'a JsWord,
    unresolved_mark: Mark,
    found: bool,
  }

  impl Visit for BindingIdentFinder<'_> {
    noop_visit_type!();

    fn visit_binding_ident(&mut self, ident: &BindingIdent) {
      if ident.id.sym == *self.sym && is_unresolved(ident, self.unresolved_mark) {
        self.found = true;
      }
    }
  }

  let mut visitor = BindingIdentFinder {
    sym,
    unresolved_mark,
    found: false,
  };
  node.visit_with(&mut visitor);
  visitor.found
}

#[cfg(test)]
mod tests {
  use super::*;

  use atlaspack_swc_runner::test_utils::{run_test_visit_const, RunVisitResult};
  use swc_core::common::Mark;

  #[test]
  fn sets_is_empty_on_empty_file() {
    assert!(run_collect("").is_empty_or_empty_export);
  }

  #[test]
  fn sets_is_empty_on_empty_export() {
    assert!(run_collect("export {};").is_empty_or_empty_export);
  }

  #[test]
  fn does_not_set_empty_on_file_with_content() {
    assert!(!run_collect("console.log('hello');").is_empty_or_empty_export);
    assert!(!run_collect("console.log('hello');console.log('world');").is_empty_or_empty_export);
  }

  #[test]
  fn does_not_set_empty_on_file_with_non_empty_export() {
    assert!(!run_collect("export default 1;").is_empty_or_empty_export);
    assert!(!run_collect("export default {};").is_empty_or_empty_export);
    assert!(!run_collect("export const a = 1;").is_empty_or_empty_export);
  }

  #[test]
  fn collects_imports() {
    assert_eq!(
      map_imports(run_collect("import { foo } from 'other';").imports),
      HashMap::from([(
        js_word!("foo"),
        PartialImport::new(ImportKind::Import, js_word!("other"), js_word!("foo")),
      )]),
    );

    assert_eq!(
      map_imports(run_collect("import { foo as bar } from 'other';").imports),
      HashMap::from([(
        js_word!("bar"),
        PartialImport::new(ImportKind::Import, js_word!("other"), js_word!("foo")),
      )]),
    );

    assert_eq!(
      map_imports(run_collect("const x = require('other');").imports),
      HashMap::from([(
        js_word!("x"),
        PartialImport::new(ImportKind::Require, js_word!("other"), js_word!("*")),
      )]),
    );

    assert_eq!(
      map_imports(run_collect("const {foo: bar} = require('other');").imports),
      HashMap::from([(
        js_word!("bar"),
        PartialImport::new(ImportKind::Require, js_word!("other"), js_word!("foo")),
      )]),
    );

    assert_eq!(
      map_imports(
        run_collect(
          "
            import { a, b, c, d, e } from 'other';
            import * as x from 'other';
            import * as y from 'other';
          "
        )
        .imports
      ),
      ["a", "b", "c", "d", "e", "x", "y"]
        .into_iter()
        .map(|s| {
          (
            JsWord::from(s),
            PartialImport {
              kind: ImportKind::Import,
              source: js_word!("other"),
              specifier: match s {
                "x" | "y" => JsWord::from("*"),
                _ => JsWord::from(s),
              },
            },
          )
        })
        .collect::<HashMap<JsWord, PartialImport>>()
    );
  }

  #[test]
  fn collects_dynamic_imports() {
    fn assert_dynamic_import(
      input_code: &str,
      imports: HashMap<JsWord, PartialImport>,
      non_static_access: HashSet<JsWord>,
      non_static_requires: HashSet<JsWord>,
    ) {
      let collect = run_collect(input_code);

      assert_eq!(map_imports(collect.imports), imports);
      assert_eq!(collect.non_static_requires, non_static_requires);
      assert_eq!(
        map_non_static_access(collect.non_static_access),
        non_static_access
      );

      assert_eq!(
        collect.wrapped_requires,
        HashSet::from([String::from("other")])
      );
    }

    assert_dynamic_import(
      "
        async function test() {
          const x = await import('other');
          x.foo;
        }
      ",
      HashMap::from([(js_word!("x"), star_import())]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "
        async function test() {
          const x = await import('other');
          x[foo];
        }
      ",
      HashMap::from([(js_word!("x"), star_import())]),
      HashSet::from([js_word!("x")]),
      HashSet::new(),
    );

    assert_dynamic_import(
      "
        async function test() {
          const {foo} = await import('other');
        }
      ",
      HashMap::from([(js_word!("foo"), foo_import())]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "
        async function test() {
          const {foo: bar} = await import('other');
        }
      ",
      HashMap::from([(js_word!("bar"), foo_import())]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(x => x.foo);",
      HashMap::from([(
        js_word!("x"),
        PartialImport {
          kind: ImportKind::DynamicImport,
          source: js_word!("other"),
          specifier: js_word!("*"),
        },
      )]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(x => x);",
      HashMap::from([(js_word!("x"), star_import())]),
      HashSet::from([js_word!("x")]),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(({foo}) => foo);",
      HashMap::from([(js_word!("foo"), foo_import())]),
      HashSet::from([js_word!("foo")]),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(({foo: bar}) => bar);",
      HashMap::from([(js_word!("bar"), foo_import())]),
      HashSet::from([js_word!("bar")]),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(function (x) { return x.foo });",
      HashMap::from([(js_word!("x"), star_import())]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(function (x) { return x });",
      HashMap::from([(js_word!("x"), star_import())]),
      HashSet::from([js_word!("x")]),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(function ({foo}) {});",
      HashMap::from([(js_word!("foo"), foo_import())]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other').then(function ({foo: bar}) {});",
      HashMap::from([(js_word!("bar"), foo_import())]),
      HashSet::new(),
      HashSet::new(),
    );

    assert_dynamic_import(
      "import('other');",
      HashMap::new(),
      HashSet::new(),
      HashSet::from([js_word!("other")]),
    );

    assert_dynamic_import(
      "let other = import('other');",
      HashMap::new(),
      HashSet::new(),
      HashSet::from([js_word!("other")]),
    );

    assert_dynamic_import(
      "
        async function test() {
          let {...other} = await import('other');
        }
      ",
      HashMap::new(),
      HashSet::new(),
      HashSet::from([js_word!("other")]),
    );

    fn foo_import() -> PartialImport {
      PartialImport::new(
        ImportKind::DynamicImport,
        js_word!("other"),
        js_word!("foo"),
      )
    }

    fn star_import() -> PartialImport {
      PartialImport::new(ImportKind::DynamicImport, js_word!("other"), js_word!("*"))
    }
  }

  #[test]
  fn collects_used_imports() {
    assert_eq!(
      map_used_imports(
        run_collect(
          "
            import { a, b, c, d, e } from 'other';
            import * as x from 'other';
            import * as y from 'other';

            log(a);
            b.x();
            c();
            log(x);
            y.foo();
            e.foo.bar();
          ",
        )
        .used_imports
      ),
      HashSet::from([
        js_word!("a"),
        js_word!("b"),
        js_word!("c"),
        js_word!("e"),
        js_word!("x"),
        js_word!("y")
      ])
    );

    assert_eq!(
      map_used_imports(
        run_collect(
          "
            import { SOURCES_CONFIG } from 'sources';
            export const getSource = SOURCES_CONFIG['static' + 'key'];
          "
        )
        .used_imports
      ),
      HashSet::from([js_word!("SOURCES_CONFIG")]),
    );

    assert_eq!(
      map_used_imports(
        run_collect(
          "
            import {bar} from 'source';

            export function thing(props) {
              const {something = bar} = props;
              return something;
            }
          ",
        )
        .used_imports
      ),
      HashSet::from([js_word!("bar")])
    );
  }

  #[test]
  fn collects_exports() {
    assert_eq!(
      run_collect("export function test() {};").exports,
      HashMap::from([(
        js_word!("test"),
        Export {
          source: None,
          specifier: "test".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 17,
            end_line: 1,
            end_col: 21
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export default function() {};").exports,
      HashMap::from([(
        js_word!("default"),
        Export {
          source: None,
          specifier: "default".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 29
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export default function test() {};").exports,
      HashMap::from([(
        js_word!("default"),
        Export {
          source: None,
          specifier: "test".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 34
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export default class {};").exports,
      HashMap::from([(
        js_word!("default"),
        Export {
          source: None,
          specifier: "default".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 24
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export default class Test {};").exports,
      HashMap::from([(
        js_word!("default"),
        Export {
          source: None,
          specifier: "Test".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 29
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export default foo;").exports,
      HashMap::from([(
        js_word!("default"),
        Export {
          source: None,
          specifier: "default".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 20
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export { foo as test };").exports,
      HashMap::from([(
        js_word!("test"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 17,
            end_line: 1,
            end_col: 21
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("export const foo = 1;").exports,
      HashMap::from([(
        js_word!("foo"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 14,
            end_line: 1,
            end_col: 17
          },
          is_esm: true
        }
      )])
    );

    assert_eq!(
      run_collect("module.exports.foo = 1;").exports,
      HashMap::from([(
        js_word!("foo"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 16,
            end_line: 1,
            end_col: 19
          },
          is_esm: false
        }
      )])
    );

    assert_eq!(
      run_collect("module.exports['foo'] = 1;").exports,
      HashMap::from([(
        js_word!("foo"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 16,
            end_line: 1,
            end_col: 21
          },
          is_esm: false
        }
      )])
    );

    assert_eq!(
      run_collect("module.exports[`foo`] = 1;").exports,
      HashMap::from([(
        js_word!("foo"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 16,
            end_line: 1,
            end_col: 21
          },
          is_esm: false
        }
      )])
    );

    assert_eq!(
      run_collect("exports.foo = 1;").exports,
      HashMap::from([(
        js_word!("foo"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 9,
            end_line: 1,
            end_col: 12
          },
          is_esm: false
        }
      )])
    );

    assert_eq!(
      run_collect("this.foo = 1;").exports,
      HashMap::from([(
        js_word!("foo"),
        Export {
          source: None,
          specifier: "foo".into(),
          loc: SourceLocation {
            start_line: 1,
            start_col: 6,
            end_line: 1,
            end_col: 9
          },
          is_esm: false
        }
      )])
    );
  }

  #[test]
  fn collects_bailouts() {
    fn assert_empty_bailouts(input_code: &str) {
      assert_eq!(run_collect(input_code).bailouts, Some(Vec::new()));
    }

    assert_empty_bailouts(
      "
        import {foo as bar} from 'other';
        let test = {bar: 3};
        console.log(bar, test.bar);
        bar();
      ",
    );

    assert_empty_bailouts(
      "
          import * as foo from 'other';
          console.log(foo.bar);
          foo.bar();
        ",
    );

    assert_empty_bailouts(
      "
          import other from 'other';
          console.log(other, other.bar);
          other();
        ",
    );

    assert_empty_bailouts(
      "
        class Foo {
          constructor() {
            this.a = 4
          }

          bar() {
            return this.baz()
          }

          baz() {
            return this.a
          }
        }

        exports.baz = new Foo()
        exports.a = 2
      ",
    );

    fn assert_bailouts(input_code: &str, bailouts: Vec<BailoutReason>) {
      assert_eq!(
        run_collect(input_code).bailouts.map(|bailouts| bailouts
          .into_iter()
          .map(|bailout| bailout.reason)
          .collect::<Vec<BailoutReason>>()),
        Some(bailouts)
      );
    }

    assert_bailouts(
      "
        import * as foo from 'other';
        foo.bar();
        let y = 'bar';
        foo[y]();
      ",
      vec![BailoutReason::NonStaticAccess],
    );

    assert_bailouts(
      "
        exports.foo = function() {
          exports.bar();
        }

        exports.bar = function() {
          this.baz();
        }

        exports.baz = function() {
          return 2;
        }
      ",
      vec![BailoutReason::ThisInExport],
    );
  }

  #[test]
  fn collects_non_static_access_requires() {
    fn assert_non_static_access(input_code: &str, non_static_access: HashSet<JsWord>) {
      assert_eq!(
        map_non_static_access(run_collect(input_code).non_static_access),
        non_static_access
      );
    }

    assert_non_static_access(
      "
        const x = require('other');
        console.log(x.foo);
      ",
      HashSet::new(),
    );

    assert_non_static_access(
      "
        const x = require('other');
        console.log(x[foo]);
      ",
      HashSet::from([js_word!("x")]),
    );

    assert_non_static_access(
      "
        const x = require('other');
        console.log(x);
      ",
      HashSet::from([js_word!("x")]),
    );
  }

  #[test]
  fn collects_has_cjs_exports() {
    fn assert_does_not_have_cjs_exports(input_code: &str) {
      assert!(!run_collect(input_code).has_cjs_exports);
    }

    // Some TSC polyfills use a pattern like below, we want to avoid marking these modules as cjs.
    assert_does_not_have_cjs_exports(
      "
        import 'something';
        var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function () {}
      ",
    );

    assert_does_not_have_cjs_exports(
      "
        const performance = module.require('perf_hooks');
        export { performance };
      ",
    );

    fn assert_has_cjs_exports(input_code: &str) {
      assert!(run_collect(input_code).has_cjs_exports);
    }

    assert_has_cjs_exports("module.exports = {};");
    assert_has_cjs_exports("this.someExport = 'true';");

    // A free module is maybe considered a cjs export
    assert_has_cjs_exports(
      "
        const performance = req(module, 'perf_hooks');
        export { performance };
      ",
    );
  }

  #[test]
  fn collects_should_wrap() {
    fn assert_should_not_wrap(input_code: &str) {
      assert!(!run_collect(input_code).should_wrap);
    }

    assert_should_not_wrap("class Foo {}");
    assert_should_not_wrap(
      "
        console.log(typeof module);
        console.log(module.hot);
      ",
    );

    assert_should_not_wrap(
      "
        const foo = {
          get a() {
            return 1;
          }
        };
        console.log(foo.a);
      ",
    );

    fn assert_should_wrap(input_code: &str) {
      assert!(run_collect(input_code).should_wrap);
    }

    assert_should_wrap("eval('');");
    assert_should_wrap("doSomething(module);");
    assert_should_wrap("console.log(module.id);");
    assert_should_wrap("exports = 1;");
    assert_should_wrap("module = 1;");

    assert_should_wrap(
      "
        console.log(module);
        export default class X {}
      ",
    );

    assert_should_wrap(
      "
        exports.foo = 1;
        return;
        exports.bar = 3;
      ",
    );

    // Module is wrapped when `this` accessor matches an export
    assert_should_wrap(
      "
        exports.foo = function() {
          exports.bar()
        }

        exports.bar = function() {
          this.baz();
        }

        exports.baz = function() {
          return 2;
        }
      ",
    );
  }

  #[test]
  fn collects_static_cjs_exports() {
    fn assert_no_static_cjs_exports(input_code: &str) {
      assert!(!run_collect(input_code).static_cjs_exports);
    }

    assert_no_static_cjs_exports("exports[test] = 1;");
    assert_no_static_cjs_exports("module.exports[test] = 1;");
    assert_no_static_cjs_exports("this[test] = 1;");
    assert_no_static_cjs_exports("alert(exports);");
    assert_no_static_cjs_exports("alert(module.exports);");
    assert_no_static_cjs_exports("alert(this);");

    fn assert_static_cjs_exports(input_code: &str) {
      assert!(run_collect(input_code).static_cjs_exports);
    }

    assert_static_cjs_exports("exports.foo = 1;");
    assert_static_cjs_exports("module.exports.foo = 1;");
    assert_static_cjs_exports("this.foo = 1;");
    assert_static_cjs_exports("test(function(exports) { return Object.keys(exports) })");
    assert_static_cjs_exports("test(exports => Object.keys(exports))");
    assert_static_cjs_exports(
      "
        var exports = {};
        exports[foo] = 1;
      ",
    );

    assert_static_cjs_exports(
      "
        var module = {exports: {}};
        module.exports[foo] = 1;
      ",
    );

    assert_static_cjs_exports(
      "
        const foo = {};
        exports.test = foo;
      ",
    );
  }

  #[test]
  fn collects_wrapped_requires() {
    fn assert_wrapped_requires(input_code: &str, wrapped_requires: HashSet<String>) {
      assert_eq!(run_collect(input_code).wrapped_requires, wrapped_requires);
    }

    assert_wrapped_requires(
      "
        function x() {
          const foo = require('other');
          console.log(foo.bar);
        }
        require('bar');
      ",
      HashSet::<String>::from_iter(vec![String::from("other")]),
    );

    assert_wrapped_requires(
      "
        var foo = function () {
          if (Date.now() < 0) {
            var bar = require('other');
          }
        }();
      ",
      HashSet::<String>::from_iter(vec![String::from("other")]),
    );

    assert_wrapped_requires(
      "
        function x() {
          const foo = require('other').foo;
          console.log(foo);
        }
      ",
      HashSet::<String>::from_iter(vec![String::from("other")]),
    );

    assert_wrapped_requires(
      "
        function x() {
          console.log(require('other').foo);
        }
      ",
      HashSet::<String>::from_iter(vec![String::from("other")]),
    );

    assert_wrapped_requires(
      "
        function x() {
          const foo = require('other')[test];
          console.log(foo);
        }
      ",
      HashSet::<String>::from_iter(vec![String::from("other")]),
    );

    assert_wrapped_requires(
      "
        function x() {
          const {foo} = require('other');
          console.log(foo);
        }
      ",
      HashSet::<String>::from_iter(vec![String::from("other")]),
    );

    assert_wrapped_requires(
      "let x = require('a') + require('b');",
      HashSet::<String>::from_iter(vec![String::from("a"), String::from("b")]),
    );

    assert_wrapped_requires(
      "let x = (require('a'), require('b'));",
      HashSet::<String>::from_iter(vec![String::from("a"), String::from("b")]),
    );

    assert_wrapped_requires(
      "let x = require('a') || require('b');",
      HashSet::<String>::from_iter(vec![String::from("a"), String::from("b")]),
    );

    assert_wrapped_requires(
      "let x = condition ? require('a') : require('b');",
      HashSet::<String>::from_iter(vec![String::from("a"), String::from("b")]),
    );

    assert_wrapped_requires(
      "if (condition) require('a');",
      HashSet::<String>::from_iter(vec![String::from("a")]),
    );

    assert_wrapped_requires(
      "for (let x = require('y'); x < 5; x++) {}",
      HashSet::<String>::from_iter(vec![String::from("y")]),
    );
  }

  fn run_collect(input_code: &str) -> Collect {
    let RunVisitResult { visitor, .. } = run_test_visit_const(input_code, |context| {
      Collect::new(
        context.source_map,
        context.unresolved_mark,
        Mark::fresh(Mark::root()),
        context.global_mark,
        true,
        context.is_module,
        false,
        true,
      )
    });

    visitor
  }

  fn map_imports(imports: HashMap<Id, Import>) -> HashMap<JsWord, PartialImport> {
    let mut map: HashMap<JsWord, PartialImport> = HashMap::new();
    for (key, import) in imports.into_iter() {
      map.insert(key.0, PartialImport::from(import));
    }

    map
  }

  fn map_non_static_access(non_static_access: HashMap<Id, Vec<Span>>) -> HashSet<JsWord> {
    non_static_access
      .into_keys()
      .map(|key| key.0)
      .collect::<HashSet<JsWord>>()
  }

  fn map_used_imports(set: HashSet<Id>) -> HashSet<JsWord> {
    set.into_iter().map(|x| x.0).collect()
  }

  #[derive(Debug, Eq, Hash, PartialEq)]
  struct PartialImport {
    kind: ImportKind,
    source: JsWord,
    specifier: JsWord,
  }

  impl PartialImport {
    pub fn new(kind: ImportKind, source: JsWord, specifier: JsWord) -> Self {
      PartialImport {
        kind,
        source,
        specifier,
      }
    }
  }

  impl From<Import> for PartialImport {
    fn from(import: Import) -> Self {
      PartialImport {
        kind: import.kind,
        source: import.source,
        specifier: import.specifier,
      }
    }
  }
}
