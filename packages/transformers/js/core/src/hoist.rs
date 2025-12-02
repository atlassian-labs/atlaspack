use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use indexmap::IndexMap;
use serde::Deserialize;
use serde::Serialize;
use swc_core::common::DUMMY_SP;
use swc_core::common::Mark;
use swc_core::common::Span;
use swc_core::common::SyntaxContext;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::Atom;
use swc_core::ecma::atoms::atom;
use swc_core::ecma::utils::stack_size::maybe_grow_default;
use swc_core::ecma::visit::Fold;
use swc_core::ecma::visit::FoldWith;

use crate::collect::Collect;
use crate::collect::Export;
use crate::collect::Import;
use crate::collect::ImportKind;
use crate::id;
use crate::utils::CodeHighlight;
use crate::utils::Diagnostic;
use crate::utils::DiagnosticSeverity;
use crate::utils::SourceLocation;
use crate::utils::get_undefined_ident;
use crate::utils::is_unresolved;
use crate::utils::match_export_name;
use crate::utils::match_export_name_ident;
use crate::utils::match_import;
use crate::utils::match_import_cond;
use crate::utils::match_member_expr;
use crate::utils::match_property_name;
use crate::utils::match_require;

macro_rules! hash {
  ($str:expr) => {{
    let mut hasher = DefaultHasher::new();
    hasher.write($str.as_bytes());
    hasher.finish()
  }};
}

pub fn hoist(
  module: Module,
  module_id: &str,
  unresolved_mark: Mark,
  collect: &Collect,
) -> Result<(Module, HoistResult, Vec<Diagnostic>), Vec<Diagnostic>> {
  let mut hoist = Hoist::new(module_id, unresolved_mark, collect);
  let module = module.fold_with(&mut hoist);

  if !hoist.diagnostics.is_empty() {
    return Err(hoist.diagnostics);
  }

  let diagnostics = std::mem::take(&mut hoist.diagnostics);
  Ok((module, hoist.get_result(), diagnostics))
}

/// An exported identifier with its original name and new mangled name.
///
/// When a file exports a symbol, atlaspack will rewrite it as a mangled
/// export identifier.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportedSymbol {
  /// The mangled name the transformer has generated and replaced the variable
  /// uses with
  pub local: Atom,
  /// The original source name that was exported
  pub exported: Atom,
  /// The location of this export
  pub loc: SourceLocation,
  pub is_esm: bool,
  pub is_static_binding_safe: bool,
}

/// An imported identifier with its rename and original name
///
/// For example, if an ESM module import is seen:
///
/// ```skip
/// import { something } from './dependency-source';
/// ```
///
/// The transformer will replace this import statement with a mangled identififer.
///
/// * `source` will be `'./dependency-source'`
/// * `imported` will be `something`
/// * `local` will usually be a mangled name the transformer has generated and replaced the
///   call-site with - except for re-exports, in which case it's just the rename
/// * `loc` will be this source-code location
///
/// See [`HoistResult::imported_symbols`] and [`HoistResult::re_exports`].
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedSymbol {
  /// The specifier for a certain dependency this symbol comes from
  pub source: Atom,
  /// The (usually mangled) local name for a certain imported symbol
  ///
  /// On re-exports, this is rather the rename for the import. See `HoistResult::re_exports`.
  pub local: Atom,
  /// The original name for a certain imported symbol
  pub imported: Atom,
  /// A location in the import site
  pub loc: SourceLocation,
  /// The type of import this symbol is coming from
  kind: ImportKind,
}

/// See [`HoistResult`] for field documentation.
struct Hoist<'a> {
  module_id: &'a str,
  collect: &'a Collect,
  module_items: Vec<ModuleItem>,
  export_decls: HashSet<Atom>,
  hoisted_imports: IndexMap<Atom, ModuleItem>,
  /// See [`HoistResult::imported_symbols`]
  imported_symbols: Vec<ImportedSymbol>,
  /// See [`HoistResult::exported_symbols`]
  exported_symbols: Vec<ExportedSymbol>,
  re_exports: Vec<ImportedSymbol>,
  /// See [`HoistResult::self_references`]
  self_references: HashSet<Atom>,
  /// See [`HoistResult::dynamic_imports`]
  dynamic_imports: HashMap<Atom, Atom>,
  in_function_scope: bool,
  diagnostics: Vec<Diagnostic>,
  unresolved_mark: Mark,
}

/// Data pertaining to mangled identifiers replacing import and export statements
/// on transformed files.
#[derive(Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HoistResult {
  /// A vector of the symbols imported from other files.
  ///
  /// For example, if a source file is:
  ///
  /// ```skip
  /// import { value as v1 } from './dependency-1';
  /// import { value as v2 } from './dependency-2';
  ///
  /// function main() {
  ///     console.log(v1);
  ///     console.log(v2);
  /// }
  /// ```
  ///
  /// The transformer will replace all usages of `v1` and `v2` with a mangled generated name.
  /// For example, the output will look like:
  ///
  /// ```skip
  /// import './dependency-1';
  /// import './dependency-2';
  ///
  /// function main() {
  ///     console.log((0, $abc$import$hashfashdfashdfahsdfh_v1));
  ///     console.log((0, $abc$import$hashfashdfashdfahsdfh_v2));
  /// }
  /// ```
  ///
  ///
  /// This `imported_symbols` vector will be:
  ///
  /// ```skip
  /// vec![
  ///     ImportedSymbol {
  ///         source: "dependency-1",
  ///         local: "$abc$import$hashfashdfashdfahsdfh_v1",
  ///         imported: "value",
  ///         ...
  ///     },
  ///     ImportedSymbol {
  ///         source: "dependency-2",
  ///         local: "$abc$import$hashfashdfashdfahsdfh_v2",
  ///         imported: "value",
  ///         ...
  ///     },
  /// ]
  /// ```
  ///
  /// `local` will be the manged name of the variables.
  pub imported_symbols: Vec<ImportedSymbol>,
  /// A vector of the symbols exported from this file, along with their mangled replacement
  /// identifiers.
  ///
  /// For example, if a source file is:
  ///
  /// ```skip
  /// export const x = 1234;
  /// export function something() {}
  /// ```
  ///
  /// The transformer will replace all usages of `x` and `something` with a mangled generated name.
  /// For example, the output will look like:
  ///
  /// ```skip
  /// const $abc$export$hashfashdfasdfahsdfh_x = 1234;
  /// function $abc$export$hashfashdfasdfahsdfh_something() {}
  /// ```
  ///
  ///
  /// This `exported_symbols` vector will be:
  ///
  /// ```skip
  /// vec![
  ///     ExportedSymbol {
  ///         exported: "x",
  ///         local: "$abc$export$hashfashdfashdfahsdfh_x",
  ///         ...
  ///     },
  ///     ExportedSymbol {
  ///         exported: "something",
  ///         local: "$abc$export$hashfashdfashdfahsdfh_something",
  ///         ...
  ///     },
  /// ]
  /// ```
  pub exported_symbols: Vec<ExportedSymbol>,
  /// Symbols re-exported from other modules.
  ///
  /// If a symbol is re-exported from another module, atlaspack will remove the export statement
  /// from the asset.
  ///
  /// For example, if an input file is:
  ///
  /// ```skip
  /// export { view as mainView } from './view';
  /// ```
  ///
  /// The output will be
  /// ```skip
  /// import 'abc:./view:esm';
  /// ```
  ///
  /// And this vector will contain the information about the re-exported symbol.
  ///
  /// On this case, the fields of `ImportedSymbol` will mean different things than they do for
  /// [`HoistResult::imported_symbols`].
  ///
  /// In particular, since there is no mangled name, `local` means the "exported" name rather than
  /// the mangled name.
  ///
  /// On the case above, this field would be:
  ///
  /// ```skip
  /// vec![
  ///     ImportedSymbol {
  ///         source: "./view",
  ///         local: "mainView",
  ///         imported: "view",
  ///         ...
  ///     },
  /// ]
  /// ```
  ///
  /// On the case the export statement is an export star:
  /// ```skip
  /// export * from './something';
  /// ```
  ///
  /// Then this array will have both `imported` and `local` set to a magic "*" value.
  pub re_exports: Vec<ImportedSymbol>,
  /// A vector of the 'original local' names of exported symbols that are self-referenced within the
  /// file they are being exported from.
  ///
  /// For example, if a file is:
  /// ```skip
  /// exports.foo = 10;
  /// exports.something = function() {
  ///     return exports.foo;
  /// }
  /// ```
  ///
  /// `self_references` will contain the `foo` symbol, un-mangled. Note the output will be mangled:
  /// ```skip
  /// var $abc$export$6a5cdcad01c973fa;
  /// var $abc$export$ce14ccb78c97a7d4;
  /// $abc$export$6a5cdcad01c973fa = 10;
  /// $abc$export$ce14ccb78c97a7d4 = function() {
  ///     return $abc$export$6a5cdcad01c973fa;
  /// };
  /// ```
  pub self_references: HashSet<Atom>,
  /// When require statements are used programmatically, their sources will be collected here.
  ///
  /// These would be the module names of dynamically imported or required modules.
  ///
  /// TODO: add example
  pub wrapped_requires: HashSet<String>,
  /// A map of async import placeholder variable names to source specifiers.
  ///
  /// When a dynamic import expression is found in the input file (`import('dependency')`), it is
  /// replaced with a generated identifier.
  ///
  /// This output field contains a map of the generated placeholder variable to the dependency
  /// specifier (`'dependency'`).
  ///
  /// For example, if the source file is
  ///
  /// ```skip
  /// async function run() {
  ///     const viewModule = await import('./view');
  ///     viewModule.render();
  /// }
  /// ```
  ///
  /// And the `module_id` of this file is `"moduleId"`, then the transformer will replace this dynamic
  /// import (assume `12345` is a hash of the `'view'` value):
  ///
  /// ```skip
  /// async function run() {
  ///     const viewModule = await $moduleId$importAsync$12345;
  ///     viewModule.render();
  /// }
  /// ```
  ///
  /// The `dynamic_imports` field will then be:
  ///
  /// ```skip
  /// {
  ///     "$moduleId$importAsync$12345": "./view"
  /// }
  /// ```
  ///
  /// In other words, the keys are the generated identifier names, inserted by the transformer and
  /// the values, the specifiers on the original source code.
  pub dynamic_imports: HashMap<Atom, Atom>,
  pub static_cjs_exports: bool,
  pub has_cjs_exports: bool,
  pub is_esm: bool,
  pub should_wrap: bool,
}

impl<'a> Hoist<'a> {
  fn new(module_id: &'a str, unresolved_mark: Mark, collect: &'a Collect) -> Self {
    Hoist {
      module_id,
      collect,
      module_items: vec![],
      export_decls: HashSet::new(),
      hoisted_imports: IndexMap::new(),
      imported_symbols: vec![],
      exported_symbols: vec![],
      re_exports: vec![],
      self_references: HashSet::new(),
      dynamic_imports: HashMap::new(),
      in_function_scope: false,
      diagnostics: vec![],
      unresolved_mark,
    }
  }

  fn get_result(self) -> HoistResult {
    HoistResult {
      imported_symbols: self.imported_symbols,
      exported_symbols: self.exported_symbols,
      re_exports: self.re_exports,
      self_references: self.self_references,
      dynamic_imports: self.dynamic_imports,
      wrapped_requires: self.collect.wrapped_requires.clone(),
      static_cjs_exports: self.collect.static_cjs_exports,
      has_cjs_exports: self.collect.has_cjs_exports,
      is_esm: self.collect.is_esm,
      should_wrap: self.collect.should_wrap,
    }
  }
}

macro_rules! hoist_visit_fn {
  ($name:ident, $type:ident) => {
    fn $name(&mut self, node: $type) -> $type {
      let in_function_scope = self.in_function_scope;
      self.in_function_scope = true;
      let res = node.fold_children_with(self);
      self.in_function_scope = in_function_scope;
      res
    }
  };
}

impl Fold for Hoist<'_> {
  fn fold_module(&mut self, node: Module) -> Module {
    let mut node = node;
    for item in node.body {
      match item {
        ModuleItem::ModuleDecl(decl) => {
          match decl {
            ModuleDecl::Import(import) => {
              self.hoisted_imports.insert(
                import.src.value.clone(),
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                  specifiers: vec![],
                  with: None,
                  span: DUMMY_SP,
                  src: Box::new(
                    format!("{}:{}:{}", self.module_id, import.src.value, "esm").into(),
                  ),
                  type_only: false,
                  phase: Default::default(),
                })),
              );
              // Ensure that all import specifiers are constant.
              for specifier in &import.specifiers {
                let local = match specifier {
                  ImportSpecifier::Named(named) => &named.local,
                  ImportSpecifier::Default(default) => &default.local,
                  ImportSpecifier::Namespace(ns) => &ns.local,
                };

                if let Some(spans) = self.collect.non_const_bindings.get(&id!(local)) {
                  let mut highlights: Vec<CodeHighlight> = spans
                    .iter()
                    .map(|span| CodeHighlight {
                      loc: SourceLocation::from(&self.collect.source_map, *span),
                      message: None,
                    })
                    .collect();

                  highlights.push(CodeHighlight {
                    loc: SourceLocation::from(&self.collect.source_map, local.span),
                    message: Some("Originally imported here".into()),
                  });

                  self.diagnostics.push(Diagnostic {
                    message: "Assignment to an import specifier is not allowed".into(),
                    code_highlights: Some(highlights),
                    hints: None,
                    show_environment: false,
                    severity: DiagnosticSeverity::Error,
                    documentation_url: None,
                  })
                }
              }
            }
            ModuleDecl::ExportNamed(export) => {
              if let Some(src) = export.src {
                self.hoisted_imports.insert(
                  src.value.clone(),
                  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    specifiers: vec![],
                    with: None,
                    span: DUMMY_SP,
                    src: Box::new(Str {
                      value: format!("{}:{}:{}", self.module_id, src.value, "esm").into(),
                      span: DUMMY_SP,
                      raw: None,
                    }),
                    type_only: false,
                    phase: Default::default(),
                  })),
                );

                for specifier in export.specifiers {
                  match specifier {
                    ExportSpecifier::Named(named) => {
                      let exported = match named.exported {
                        Some(exported) => match_export_name(&exported).0,
                        None => match_export_name(&named.orig).0.clone(),
                      };
                      self.re_exports.push(ImportedSymbol {
                        source: src.value.clone(),
                        local: exported,
                        imported: match_export_name(&named.orig).0,
                        loc: SourceLocation::from(&self.collect.source_map, named.span),
                        kind: ImportKind::Import,
                      });
                    }
                    ExportSpecifier::Default(default) => {
                      self.re_exports.push(ImportedSymbol {
                        source: src.value.clone(),
                        local: default.exported.sym,
                        imported: atom!("default"),
                        loc: SourceLocation::from(&self.collect.source_map, default.exported.span),
                        kind: ImportKind::Import,
                      });
                    }
                    ExportSpecifier::Namespace(namespace) => {
                      self.re_exports.push(ImportedSymbol {
                        source: src.value.clone(),
                        local: match_export_name(&namespace.name).0,
                        imported: "*".into(),
                        loc: SourceLocation::from(&self.collect.source_map, namespace.span),
                        kind: ImportKind::Import,
                      });
                    }
                  }
                }
              } else {
                for specifier in export.specifiers {
                  if let ExportSpecifier::Named(named) = specifier {
                    let id = id!(match_export_name_ident(&named.orig));
                    let exported_node = match named.exported {
                      Some(exported) => exported,
                      None => named.orig,
                    };
                    let exported = match_export_name(&exported_node).0;
                    if let Some(Import {
                      source,
                      specifier,
                      kind,
                      ..
                    }) = self.collect.imports.get(&id)
                    {
                      self.re_exports.push(ImportedSymbol {
                        source: source.clone(),
                        local: exported,
                        imported: specifier.clone(),
                        loc: SourceLocation::from(&self.collect.source_map, named.span),
                        kind: *kind,
                      });
                    } else {
                      // A variable will appear only once in the `exports` mapping but
                      // could be exported multiple times with different names.
                      // Find the original exported name, and remap.
                      let id = if self.collect.should_wrap {
                        id.0
                      } else {
                        self
                          .get_export_ident(DUMMY_SP, self.collect.exports_locals.get(&id).unwrap())
                          .sym
                      };

                      let is_static_binding_safe =
                        if let ModuleExportName::Ident(ident) = exported_node {
                          self
                            .collect
                            .symbols_info
                            .is_static_binding_safe(&ident.to_id())
                        } else {
                          false
                        };
                      self.exported_symbols.push(ExportedSymbol {
                        local: id.clone(),
                        exported,
                        loc: SourceLocation::from(&self.collect.source_map, named.span),
                        is_esm: true,
                        is_static_binding_safe,
                      });
                    }
                  }
                }
              }
            }
            ModuleDecl::ExportAll(export) => {
              self.hoisted_imports.insert(
                export.src.value.clone(),
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                  specifiers: vec![],
                  with: None,
                  span: DUMMY_SP,
                  src: Box::new(
                    format!("{}:{}:{}", self.module_id, export.src.value, "esm").into(),
                  ),
                  type_only: false,
                  phase: Default::default(),
                })),
              );
              self.re_exports.push(ImportedSymbol {
                source: export.src.value,
                local: "*".into(),
                imported: "*".into(),
                loc: SourceLocation::from(&self.collect.source_map, export.span),
                kind: ImportKind::Import,
              });
            }
            ModuleDecl::ExportDefaultExpr(export) => {
              let ident = self.get_export_ident(export.span, &"default".into());
              let init = export.expr.fold_with(self);
              self
                .module_items
                .push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                  declare: false,
                  kind: VarDeclKind::Var,
                  span: DUMMY_SP,
                  ctxt: SyntaxContext::empty(),
                  decls: vec![VarDeclarator {
                    definite: false,
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent::from(ident)),
                    init: Some(init),
                  }],
                })))));
            }
            ModuleDecl::ExportDefaultDecl(export) => {
              let decl = match export.decl {
                DefaultDecl::Class(class) => Decl::Class(ClassDecl {
                  ident: if self.collect.should_wrap && class.ident.is_some() {
                    class.ident.unwrap()
                  } else {
                    self.get_export_ident(DUMMY_SP, &"default".into())
                  },
                  declare: false,
                  class: class.class.fold_with(self),
                }),
                DefaultDecl::Fn(func) => Decl::Fn(FnDecl {
                  ident: if self.collect.should_wrap && func.ident.is_some() {
                    func.ident.unwrap()
                  } else {
                    self.get_export_ident(DUMMY_SP, &"default".into())
                  },
                  declare: false,
                  function: func.function.fold_with(self),
                }),
                _ => {
                  unreachable!("unsupported export default declaration");
                }
              };

              self.module_items.push(ModuleItem::Stmt(Stmt::Decl(decl)));
            }
            ModuleDecl::ExportDecl(export) => {
              let d = export.decl.fold_with(self);
              self.module_items.push(ModuleItem::Stmt(Stmt::Decl(d)));
            }
            item => {
              let d = item.fold_with(self);
              self.module_items.push(ModuleItem::ModuleDecl(d))
            }
          }
        }
        ModuleItem::Stmt(stmt) => {
          match stmt {
            Stmt::Decl(decl) => {
              match decl {
                Decl::Var(var) => {
                  let mut decls = vec![];
                  for v in &var.decls {
                    if let Some(init) = &v.init {
                      // Match var x = require('foo');
                      if let Some(source) =
                        match_require(init, self.unresolved_mark, self.collect.ignore_mark, None)
                      {
                        // If the require is accessed in a way we cannot analyze, do not replace.
                        // e.g. const {x: {y: z}} = require('x');
                        // The require will be handled in the expression handler, below.
                        if !self.collect.non_static_requires.contains(&source) {
                          // If this is not the first declarator in the variable declaration, we need to
                          // split the declaration into multiple to preserve side effect ordering.
                          // var x = sideEffect(), y = require('foo'), z = 2;
                          //   -> var x = sideEffect(); import 'foo'; var y = $id$import$foo, z = 2;
                          if !decls.is_empty() {
                            let var = VarDecl {
                              span: var.span,
                              ctxt: var.ctxt,
                              kind: var.kind,
                              declare: var.declare,
                              decls: std::mem::take(&mut decls),
                            };
                            self
                              .module_items
                              .push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var)))));
                          }

                          self
                            .module_items
                            .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                              specifiers: vec![],
                              with: None,
                              span: DUMMY_SP,
                              src: Box::new(Str {
                                value: format!("{}:{}", self.module_id, source).into(),
                                span: DUMMY_SP,
                                raw: None,
                              }),
                              type_only: false,
                              phase: Default::default(),
                            })));

                          // Create variable assignments for any declarations that are not constant.
                          self.handle_non_const_require(v, &source);
                          continue;
                        }
                      }

                      if let Expr::Member(member) = &**init {
                        // Match var x = require('foo').bar;
                        if let Some(source) = match_require(
                          &member.obj,
                          self.unresolved_mark,
                          self.collect.ignore_mark,
                          None,
                        ) && !self.collect.non_static_requires.contains(&source)
                        {
                          // If this is not the first declarator in the variable declaration, we need to
                          // split the declaration into multiple to preserve side effect ordering.
                          // var x = sideEffect(), y = require('foo').bar, z = 2;
                          //   -> var x = sideEffect(); import 'foo'; var y = $id$import$foo$bar, z = 2;
                          if !decls.is_empty() {
                            let var = VarDecl {
                              span: var.span,
                              ctxt: var.ctxt,
                              kind: var.kind,
                              declare: var.declare,
                              decls: std::mem::take(&mut decls),
                            };
                            self
                              .module_items
                              .push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var)))));
                          }
                          self
                            .module_items
                            .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                              specifiers: vec![],
                              with: None,
                              span: DUMMY_SP,
                              src: Box::new(Str {
                                value: format!("{}:{}", self.module_id, source,).into(),
                                span: DUMMY_SP,
                                raw: None,
                              }),
                              type_only: false,
                              phase: Default::default(),
                            })));

                          self.handle_non_const_require(v, &source);
                          continue;
                        }
                      }
                    }

                    // Otherwise, fold the variable initializer. If requires were found
                    // in the expression, they will be hoisted into module_items. If the
                    // length increases, then we need to split the variable declaration
                    // into multiple to preserve side effect ordering.
                    // var x = 2, y = doSomething(require('foo')), z = 3;
                    //   -> var x = 2; import 'foo'; var y = doSomething($id$import$foo), z = 3;
                    let items_len = self.module_items.len();
                    let d = v.clone().fold_with(self);
                    if self.module_items.len() > items_len && !decls.is_empty() {
                      let var = VarDecl {
                        span: var.span,
                        ctxt: var.ctxt,
                        kind: var.kind,
                        declare: var.declare,
                        decls: std::mem::take(&mut decls),
                      };
                      self.module_items.insert(
                        items_len,
                        ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var)))),
                      );
                    }
                    decls.push(d);
                  }

                  // Push whatever declarators are left.
                  if !decls.is_empty() {
                    let var = VarDecl {
                      span: var.span,
                      ctxt: var.ctxt,
                      kind: var.kind,
                      declare: var.declare,
                      decls,
                    };
                    self
                      .module_items
                      .push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var)))))
                  }
                }
                item => {
                  let d = item.fold_with(self);
                  self.module_items.push(ModuleItem::Stmt(Stmt::Decl(d)))
                }
              }
            }
            Stmt::Expr(ExprStmt { expr, span }) => {
              if let Some(source) =
                match_require(&expr, self.unresolved_mark, self.collect.ignore_mark, None)
              {
                // Require in statement position (`require('other');`) should behave just
                // like `import 'other';` in that it doesn't add any symbols (not even '*').
                self.add_require(&source, ImportKind::Require);
              } else {
                let d = expr.fold_with(self);
                self
                  .module_items
                  .push(ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr: d, span })))
              }
            }
            item => {
              let d = item.fold_with(self);
              self.module_items.push(ModuleItem::Stmt(d))
            }
          }
        }
      }
    }

    self.module_items.splice(
      0..0,
      std::mem::take(&mut self.hoisted_imports).into_values(),
    );
    node.body = std::mem::take(&mut self.module_items);
    node
  }

  hoist_visit_fn!(fold_function, Function);
  hoist_visit_fn!(fold_class, Class);
  hoist_visit_fn!(fold_getter_prop, GetterProp);
  hoist_visit_fn!(fold_setter_prop, SetterProp);

  fn fold_expr(&mut self, node: Expr) -> Expr {
    match node {
      Expr::OptChain(opt) => {
        return Expr::OptChain(OptChainExpr {
          span: opt.span,
          optional: opt.optional,
          base: Box::new(match *opt.base {
            OptChainBase::Call(call) => OptChainBase::Call(call.fold_with(self)),
            OptChainBase::Member(member) => {
              if match_property_name(&member).is_some() {
                OptChainBase::Member(MemberExpr {
                  span: member.span,
                  obj: member.obj.fold_with(self),
                  // Don't visit member.prop so we avoid the ident visitor.
                  prop: member.prop,
                })
              } else {
                OptChainBase::Member(member.fold_children_with(self))
              }
            }
          }),
        });
      }
      Expr::Member(member) => {
        if !self.collect.should_wrap {
          if match_member_expr(
            &member,
            vec!["module", "exports"],
            self.unresolved_mark,
            None,
          ) {
            self.self_references.insert("*".into());
            return Expr::Ident(self.get_export_ident(member.span, &"*".into()));
          }

          if match_member_expr(&member, vec!["module", "hot"], self.unresolved_mark, None) {
            return Expr::Lit(Lit::Null(Null { span: member.span }));
          }
        }

        let key = match match_property_name(&member) {
          Some(v) => v.0,
          _ => return Expr::Member(member.fold_children_with(self)),
        };

        match &*member.obj {
          Expr::Ident(ident) => {
            // import * as y from 'x'; OR const y = require('x'); OR const y = await import('x');
            // y.foo -> $id$import$d141bba7fdc215a3$y
            if let Some(Import {
              source,
              specifier,
              kind,
              ..
            }) = self.collect.imports.get(&id!(ident))
            {
              // If there are any non-static accesses of the namespace, don't perform any replacement.
              // This will be handled in the Ident visitor below, which replaces y -> $id$import$d141bba7fdc215a3.
              if specifier == "*"
                && !self.collect.non_static_access.contains_key(&id!(ident))
                && !self.collect.non_const_bindings.contains_key(&id!(ident))
                && !self.collect.non_static_requires.contains(source)
              {
                if *kind == ImportKind::DynamicImport {
                  let name: Atom = format!(
                    "${}$importAsync${:x}${:x}",
                    self.module_id,
                    hash!(source),
                    hash!(key)
                  )
                  .into();
                  self.imported_symbols.push(ImportedSymbol {
                    source: source.clone(),
                    local: name,
                    imported: key.clone(),
                    loc: SourceLocation::from(&self.collect.source_map, member.span),
                    kind: *kind,
                  });
                } else {
                  return Expr::Ident(self.get_import_ident(
                    member.span,
                    source,
                    &key,
                    SourceLocation::from(&self.collect.source_map, member.span),
                    *kind,
                  ));
                }
              }
            }

            // exports.foo -> $id$export$foo
            if &*ident.sym == "exports"
              && is_unresolved(ident, self.unresolved_mark)
              && self.collect.static_cjs_exports
              && !self.collect.should_wrap
            {
              self.self_references.insert(key.clone());
              return Expr::Ident(self.get_export_ident(member.span, &key));
            }
          }
          Expr::Call(_) => {
            // require('foo').bar -> $id$import$foo$bar
            if let Some(source) = match_require(
              &member.obj,
              self.unresolved_mark,
              self.collect.ignore_mark,
              None,
            ) {
              self.add_require(&source, ImportKind::Require);
              return Expr::Ident(self.get_import_ident(
                member.span,
                &source,
                &key,
                SourceLocation::from(&self.collect.source_map, member.span),
                ImportKind::Require,
              ));
            }
          }
          Expr::Member(mem) => {
            // module.exports.foo -> $id$export$foo
            if self.collect.static_cjs_exports
              && !self.collect.should_wrap
              && match_member_expr(mem, vec!["module", "exports"], self.unresolved_mark, None)
            {
              self.self_references.insert(key.clone());
              return Expr::Ident(self.get_export_ident(member.span, &key));
            }
          }
          Expr::This(_) => {
            // this.foo -> $id$export$foo
            if self.collect.static_cjs_exports
              && !self.collect.should_wrap
              && !self.in_function_scope
              && !self.collect.is_esm
            {
              self.self_references.insert(key.clone());
              return Expr::Ident(self.get_export_ident(member.span, &key));
            }
          }
          _ => {}
        }

        // Don't visit member.prop so we avoid the ident visitor.
        return Expr::Member(MemberExpr {
          span: member.span,
          obj: member.obj.fold_with(self),
          prop: member.prop,
        });
      }
      Expr::Call(ref call) => {
        // require('foo') -> $id$import$foo
        if let Some(source) =
          match_require(&node, self.unresolved_mark, self.collect.ignore_mark, None)
        {
          self.add_require(&source, ImportKind::Require);
          return Expr::Ident(self.get_import_ident(
            call.span,
            &source,
            &("*".into()),
            SourceLocation::from(&self.collect.source_map, call.span),
            ImportKind::Require,
          ));
        }

        if let Some(source) = match_import(&node) {
          self.add_require(&source, ImportKind::DynamicImport);
          let name: Atom = format!("${}$importAsync${:x}", self.module_id, hash!(source)).into();
          self.dynamic_imports.insert(name.clone(), source.clone());
          if self.collect.non_static_requires.contains(&source) || self.collect.should_wrap {
            self.imported_symbols.push(ImportedSymbol {
              source,
              local: name.clone(),
              imported: "*".into(),
              loc: SourceLocation::from(&self.collect.source_map, call.span),
              kind: ImportKind::DynamicImport,
            });
          }
          return Expr::Ident(Ident::new(name, call.span, call.ctxt));
        }

        if let Some((source_true, source_false)) =
          match_import_cond(&node, self.collect.ignore_mark)
        {
          let name: Atom = format!("${}$importCond${}", self.module_id, hash!(source_true)).into();
          self.add_require(&source_true, ImportKind::ConditionalImport);
          self.add_require(&source_false, ImportKind::ConditionalImport);

          // Mark both deps as dynamic import
          self
            .dynamic_imports
            .insert(name.clone(), source_true.clone());
          self
            .dynamic_imports
            .insert(name.clone(), source_false.clone());

          // Mark both deps "imported symbols"
          self.imported_symbols.push(ImportedSymbol {
            source: source_true,
            local: name.clone(),
            imported: "*".into(),
            loc: SourceLocation::from(&self.collect.source_map, call.span),
            kind: ImportKind::ConditionalImport,
          });
          self.imported_symbols.push(ImportedSymbol {
            source: source_false,
            local: name.clone(),
            imported: "*".into(),
            loc: SourceLocation::from(&self.collect.source_map, call.span),
            kind: ImportKind::ConditionalImport,
          });

          return Expr::Ident(Ident::new_no_ctxt(name, call.span));
        }
      }
      Expr::This(this) => {
        if !self.in_function_scope {
          // If ESM, replace `this` with `undefined`, otherwise with the CJS exports object.
          if self.collect.is_esm {
            return Expr::Ident(get_undefined_ident(self.unresolved_mark));
          } else if !self.collect.should_wrap {
            self.self_references.insert("*".into());
            return Expr::Ident(self.get_export_ident(this.span, &"*".into()));
          }
        }
      }
      Expr::Ident(ident) => {
        // import { foo } from "..."; foo();
        // ->
        // import { foo } from "..."; (0, foo)();
        if let Some(Import {
          specifier, kind, ..
        }) = self.collect.imports.get(&id!(ident))
          && kind == &ImportKind::Import
          && specifier != "*"
        {
          return Expr::Seq(SeqExpr {
            span: ident.span,
            exprs: vec![0.into(), Box::new(Expr::Ident(ident.fold_with(self)))],
          });
        }
        return Expr::Ident(ident.fold_with(self));
      }
      _ => {}
    }

    maybe_grow_default(|| node.fold_children_with(self))
  }

  fn fold_seq_expr(&mut self, node: SeqExpr) -> SeqExpr {
    // This is a hack to work around the SWC fixer pass removing identifiers in sequence expressions
    // that aren't at the end. In general this makes sense, but we need to preserve these so that they
    // can be replaced with a atlaspackRequire call in the linker. We just wrap with a unary expression to
    // get around this for now.
    let len = node.exprs.len();
    let exprs = node
      .exprs
      .into_iter()
      .enumerate()
      .map(|(i, expr)| {
        if i != len - 1
          && match_require(&expr, self.unresolved_mark, self.collect.ignore_mark, None).is_some()
        {
          return Box::new(Expr::Unary(UnaryExpr {
            op: UnaryOp::Bang,
            arg: expr.fold_with(self),
            span: DUMMY_SP,
          }));
        }

        expr.fold_with(self)
      })
      .collect();

    SeqExpr { exprs, ..node }
  }

  fn fold_ident(&mut self, node: Ident) -> Ident {
    // import {x} from 'y'; OR const {x} = require('y');
    // x -> $id$import$y$x
    //
    // import * as x from 'y'; OR const x = require('y');
    // x -> $id$import$y
    if let Some(Import {
      source,
      specifier,
      kind,
      loc,
      ..
    }) = self.collect.imports.get(&id!(node))
    {
      // If the require is accessed in a way we cannot analyze, do not replace.
      // e.g. const {x: {y: z}} = require('x');
      if !self.collect.non_static_requires.contains(source) {
        if *kind == ImportKind::DynamicImport {
          if specifier != "*" {
            let name: Atom = format!(
              "${}$importAsync${:x}${:x}",
              self.module_id,
              hash!(source),
              hash!(specifier)
            )
            .into();
            self.imported_symbols.push(ImportedSymbol {
              source: source.clone(),
              local: name,
              imported: specifier.clone(),
              loc: loc.clone(),
              kind: *kind,
            });
          } else if self.collect.non_static_access.contains_key(&id!(node)) {
            let name: Atom = format!("${}$importAsync${:x}", self.module_id, hash!(source)).into();
            self.imported_symbols.push(ImportedSymbol {
              source: source.clone(),
              local: name,
              imported: "*".into(),
              loc: loc.clone(),
              kind: *kind,
            });
          }
        } else {
          // If this identifier is not constant, we cannot directly reference the imported
          // value. Instead, a new local variable is created that originally points to the
          // required value, and we reference that instead. This allows the local variable
          // to be re-assigned without affecting the original exported variable.
          // See handle_non_const_require, below.
          if self.collect.non_const_bindings.contains_key(&id!(node)) {
            return self.get_require_ident(&node.sym);
          }

          return self.get_import_ident(node.span, source, specifier, loc.clone(), *kind);
        }
      }
    }

    if let Some(exported) = self.collect.exports_locals.get(&id!(node)) {
      // If wrapped, mark the original symbol as exported.
      // Otherwise replace with an export identifier.
      if self.collect.should_wrap {
        self.exported_symbols.push(ExportedSymbol {
          local: node.sym.clone(),
          exported: exported.clone(),
          loc: SourceLocation::from(&self.collect.source_map, node.span),
          is_esm: false,
          is_static_binding_safe: false,
        });
        return node;
      } else {
        return self.get_export_ident(node.span, exported);
      }
    }

    if &*node.sym == "exports"
      && is_unresolved(&node, self.unresolved_mark)
      && !self.collect.should_wrap
    {
      self.self_references.insert("*".into());
      return self.get_export_ident(node.span, &"*".into());
    }

    if node.sym == atom!("global") && is_unresolved(&node, self.unresolved_mark) {
      return Ident::new("$parcel$global".into(), node.span, node.ctxt);
    }

    if node.ctxt.has_mark(self.collect.global_mark)
      && !is_unresolved(&node, self.unresolved_mark)
      && !self.collect.should_wrap
    {
      let new_name: Atom = format!("${}$var${}", self.module_id, node.sym).into();
      return Ident::new(new_name, node.span, node.ctxt);
    }

    node
  }

  fn fold_assign_expr(&mut self, node: AssignExpr) -> AssignExpr {
    if self.collect.should_wrap {
      return node.fold_children_with(self);
    }

    if let AssignTarget::Simple(SimpleAssignTarget::Member(member)) = &node.left {
      if match_member_expr(
        member,
        vec!["module", "exports"],
        self.unresolved_mark,
        None,
      ) {
        let ident = BindingIdent::from(self.get_export_ident(member.span, &"*".into()));
        return AssignExpr {
          span: node.span,
          op: node.op,
          left: AssignTarget::Simple(SimpleAssignTarget::Ident(ident)),
          right: node.right.fold_with(self),
        };
      }

      let is_cjs_exports = match &*member.obj {
        Expr::Member(member) => match_member_expr(
          member,
          vec!["module", "exports"],
          self.unresolved_mark,
          None,
        ),
        Expr::Ident(ident) => {
          &*ident.sym == "exports" && is_unresolved(ident, self.unresolved_mark)
        }
        Expr::This(_) if !self.in_function_scope => true,
        _ => false,
      };

      if is_cjs_exports {
        let key: Atom = if self.collect.static_cjs_exports {
          if let Some((name, _)) = match_property_name(member) {
            name
          } else {
            unreachable!("Unexpected non-static CJS export");
          }
        } else {
          "*".into()
        };

        let ident = BindingIdent::from(self.get_export_ident(member.span, &key));
        if self.collect.static_cjs_exports && self.export_decls.insert(ident.id.sym.clone()) {
          self.hoisted_imports.insert(
            ident.id.sym.clone(),
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
              declare: false,
              kind: VarDeclKind::Var,
              span: node.span,
              ctxt: ident.ctxt,
              decls: vec![VarDeclarator {
                definite: false,
                span: node.span,
                name: Pat::Ident(BindingIdent::from(Ident::new(
                  ident.id.sym.clone(),
                  DUMMY_SP,
                  ident.ctxt,
                ))),
                init: None,
              }],
            })))),
          );
        }

        return AssignExpr {
          span: node.span,
          op: node.op,
          left: if self.collect.static_cjs_exports {
            AssignTarget::Simple(SimpleAssignTarget::Ident(ident))
          } else {
            AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
              span: member.span,
              obj: Box::new(Expr::Ident(ident.id)),
              prop: member.prop.clone().fold_with(self),
            }))
          },
          right: node.right.fold_with(self),
        };
      }
    }

    node.fold_children_with(self)
  }

  fn fold_prop(&mut self, node: Prop) -> Prop {
    match node {
      Prop::Shorthand(ident) => Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IdentName::new(ident.sym.clone(), DUMMY_SP)),
        value: Box::new(Expr::Ident(ident.fold_with(self))),
      }),
      _ => node.fold_children_with(self),
    }
  }

  fn fold_prop_name(&mut self, node: PropName) -> PropName {
    match node {
      PropName::Computed(k) => PropName::Computed(k.fold_with(self)),
      k => k,
    }
  }

  fn fold_object_pat_prop(&mut self, node: ObjectPatProp) -> ObjectPatProp {
    if self.collect.should_wrap {
      return node.fold_children_with(self);
    }

    // var {a, b} = foo; -> var {a: $id$var$a, b: $id$var$b} = foo;
    match node {
      ObjectPatProp::Assign(assign) => ObjectPatProp::KeyValue(KeyValuePatProp {
        key: PropName::Ident(IdentName::new(assign.key.sym.clone(), DUMMY_SP)),
        value: Box::new(match assign.value {
          Some(value) => Pat::Assign(AssignPat {
            left: Box::new(Pat::Ident(assign.key.fold_with(self))),
            right: value.fold_with(self),
            span: DUMMY_SP,
          }),
          None => Pat::Ident(assign.key.fold_with(self)),
        }),
      }),
      _ => node.fold_children_with(self),
    }
  }
}

impl Hoist<'_> {
  fn add_require(&mut self, source: &Atom, import_kind: ImportKind) {
    let src = match import_kind {
      ImportKind::Import => format!("{}:{}:{}", self.module_id, source, "esm"),
      ImportKind::DynamicImport | ImportKind::Require | ImportKind::ConditionalImport => {
        format!("{}:{}", self.module_id, source)
      }
    };
    self
      .module_items
      .push(ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        specifiers: vec![],
        with: None,
        span: DUMMY_SP,
        src: Box::new(src.into()),
        type_only: false,
        phase: Default::default(),
      })));
  }

  fn get_import_name(&self, source: &Atom, local: &Atom) -> Atom {
    if local == "*" {
      format!("${}$import${:x}", self.module_id, hash!(source)).into()
    } else {
      format!(
        "${}$import${:x}${:x}",
        self.module_id,
        hash!(source),
        hash!(local)
      )
      .into()
    }
  }

  fn get_import_ident(
    &mut self,
    span: Span,
    source: &Atom,
    imported: &Atom,
    loc: SourceLocation,
    kind: ImportKind,
  ) -> Ident {
    let new_name = self.get_import_name(source, imported);
    self.imported_symbols.push(ImportedSymbol {
      source: source.clone(),
      local: new_name.clone(),
      imported: imported.clone(),
      loc,
      kind,
    });
    Ident::new_no_ctxt(new_name, span)
  }

  fn get_require_ident(&self, local: &Atom) -> Ident {
    Ident::new_no_ctxt(
      format!("${}$require${}", self.module_id, local).into(),
      DUMMY_SP,
    )
  }

  fn get_export_ident(&mut self, span: Span, exported: &Atom) -> Ident {
    let new_name: Atom = if exported == "*" {
      format!("${}$exports", self.module_id).into()
    } else {
      format!("${}$export${:x}", self.module_id, hash!(exported)).into()
    };

    let is_esm = matches!(
      self.collect.exports.get(exported),
      Some(Export { is_esm: true, .. })
    );

    let is_static_binding_safe = matches!(
      self.collect.exports.get(exported),
      Some(Export {
        is_static_binding_safe: true,
        ..
      })
    );

    self.exported_symbols.push(ExportedSymbol {
      local: new_name.clone(),
      exported: exported.clone(),
      loc: SourceLocation::from(&self.collect.source_map, span),
      is_esm,
      is_static_binding_safe,
    });

    Ident::new_no_ctxt(new_name, span)
  }

  fn handle_non_const_require(&mut self, v: &VarDeclarator, source: &Atom) {
    // If any of the bindings in this declarator are not constant, we need to create
    // a local variable referencing them so that we can safely re-assign the local variable
    // without affecting the original export. This is only possible in CommonJS since ESM
    // imports are constant (this is ensured by the diagnostic in fold_module above).
    let mut non_const_bindings = vec![];
    self
      .collect
      .get_non_const_binding_idents(&v.name, &mut non_const_bindings);

    for ident in non_const_bindings {
      if let Some(Import {
        specifier, kind, ..
      }) = self.collect.imports.get(&id!(ident))
      {
        let require_id = self.get_require_ident(&ident.sym);
        let import_id = self.get_import_ident(
          v.span,
          source,
          specifier,
          SourceLocation::from(&self.collect.source_map, v.span),
          *kind,
        );
        self
          .module_items
          .push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            declare: false,
            kind: VarDeclKind::Var,
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            decls: vec![VarDeclarator {
              definite: false,
              span: DUMMY_SP,
              name: Pat::Ident(BindingIdent::from(require_id)),
              init: Some(Box::new(Expr::Ident(import_id))),
            }],
          })))));
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::test_utils::{RunVisitResult, run_test_fold, run_test_visit_const};
  use indoc::{formatdoc, indoc};

  use crate::esm_export_classifier::SymbolsInfo;

  use super::*;

  #[test]
  fn test_imports() {
    fn assert_imports(
      input_code: &str,
      expected_code: &str,
      imported_symbols: Vec<PartialImportedSymbol>,
    ) {
      let (code, hoist) = run_hoist(input_code);

      assert_eq!(code, expected_code);
      assert_eq!(
        map_imported_symbols(hoist.imported_symbols),
        imported_symbols
      );
    }

    assert_imports(
      "
        import foo from 'other';
        console.log(foo);
        import bar from 'bar';
        console.log(bar);
      ",
      indoc! {r#"
        import "abc:other:esm";
        import "abc:bar:esm";
        console.log(0, $abc$import$70a00e0a8474f72a$2e2bcd8739ae039);
        console.log(0, $abc$import$d927737047eb3867$2e2bcd8739ae039);
      "#},
      vec![other_default_symbol(), bar_default_symbol()],
    );

    assert_imports(
      "
        import foo from 'other';
        console.log(foo, foo.bar);
        console.log(foo())
        const x = require('x');
        console.log(x);
        import bar from 'bar';
        console.log(bar);
      ",
      indoc! {r#"
        import "abc:other:esm";
        import "abc:bar:esm";
        console.log(0, $abc$import$70a00e0a8474f72a$2e2bcd8739ae039, 0, $abc$import$70a00e0a8474f72a$2e2bcd8739ae039.bar);
        console.log(0, $abc$import$70a00e0a8474f72a$2e2bcd8739ae039());
        import "abc:x";
        console.log($abc$import$d141bba7fdc215a3);
        console.log(0, $abc$import$d927737047eb3867$2e2bcd8739ae039);
      "#},
      vec![
        other_default_symbol(),
        other_default_symbol(),
        other_default_symbol(),
        PartialImportedSymbol {
          imported: atom!("*"),
          kind: ImportKind::Require,
          local: atom!("$abc$import$d141bba7fdc215a3"),
          source: atom!("x"),
        },
        bar_default_symbol(),
      ],
    );

    assert_imports(
      "
        import {foo as bar} from 'other';
        let test = {bar: 3};
        console.log(bar, test.bar);
        bar();
      ",
      indoc! {r#"
        import "abc:other:esm";
        let $abc$var$test = {
            bar: 3
        };
        console.log(0, $abc$import$70a00e0a8474f72a$6a5cdcad01c973fa, $abc$var$test.bar);
        0, $abc$import$70a00e0a8474f72a$6a5cdcad01c973fa();
      "#},
      vec![other_foo_symbol(), other_foo_symbol()],
    );

    assert_imports(
      "
        import * as foo from 'other';
        console.log(foo.bar);
        foo.bar();
      ",
      indoc! {r#"
        import "abc:other:esm";
        console.log($abc$import$70a00e0a8474f72a$d927737047eb3867);
        $abc$import$70a00e0a8474f72a$d927737047eb3867();
      "#},
      vec![other_bar_symbol(), other_bar_symbol()],
    );

    assert_imports(
      "
        import { foo } from 'other';
        async function test() {
          console.log(foo.bar);
        }
      ",
      indoc! {r#"
        import "abc:other:esm";
        async function $abc$var$test() {
            console.log(0, $abc$import$70a00e0a8474f72a$6a5cdcad01c973fa.bar);
        }
      "#},
      vec![other_foo_symbol()],
    );

    fn other_foo_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("foo"),
        kind: ImportKind::Import,
        local: atom!("$abc$import$70a00e0a8474f72a$6a5cdcad01c973fa"),
        source: atom!("other"),
      }
    }

    fn other_bar_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("bar"),
        kind: ImportKind::Import,
        local: atom!("$abc$import$70a00e0a8474f72a$d927737047eb3867"),
        source: atom!("other"),
      }
    }

    fn other_default_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("default"),
        kind: ImportKind::Import,
        local: atom!("$abc$import$70a00e0a8474f72a$2e2bcd8739ae039"),
        source: atom!("other"),
      }
    }

    fn bar_default_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("default"),
        kind: ImportKind::Import,
        local: atom!("$abc$import$d927737047eb3867$2e2bcd8739ae039"),
        source: atom!("bar"),
      }
    }
  }

  #[test]
  fn test_dynamic_imports() {
    fn assert_dynamic_import(
      input_code: &str,
      expected_code: &str,
      imported_symbols: Vec<PartialImportedSymbol>,
    ) {
      let (code, hoist) = run_hoist(input_code);

      assert_eq!(code, expected_code);
      assert_eq!(
        map_imported_symbols(hoist.imported_symbols),
        imported_symbols
      );

      assert_eq!(
        hoist.dynamic_imports,
        HashMap::from([(atom!("$abc$importAsync$70a00e0a8474f72a"), atom!("other"))])
      );
    }

    assert_dynamic_import(
      "
        async function test() {
          const x = await import('other');
          console.log(x.foo);
        }
     ",
      indoc! {r#"
        import "abc:other";
        async function $abc$var$test() {
            const x = await $abc$importAsync$70a00e0a8474f72a;
            console.log(x.foo);
        }
      "#},
      vec![foo_symbol()],
    );

    assert_dynamic_import(
      "
        async function test() {
          const x = await import('other');
          console.log(x[foo]);
        }
     ",
      indoc! {r#"
        import "abc:other";
        async function $abc$var$test() {
            const x = await $abc$importAsync$70a00e0a8474f72a;
            console.log(x[foo]);
        }
      "#},
      vec![star_symbol(), star_symbol()],
    );

    assert_dynamic_import(
      "
        async function test() {
          const x = await import('other');
          console.log(foo);
        }
     ",
      indoc! {r#"
        import "abc:other";
        async function $abc$var$test() {
            const x = await $abc$importAsync$70a00e0a8474f72a;
            console.log(foo);
        }
      "#},
      vec![],
    );

    assert_dynamic_import(
      "
        async function test() {
          const {foo} = await import('other');
          console.log(foo);
        }
     ",
      indoc! {r#"
        import "abc:other";
        async function $abc$var$test() {
            const { foo: foo } = await $abc$importAsync$70a00e0a8474f72a;
            console.log(foo);
        }
      "#},
      vec![foo_symbol(), foo_symbol()],
    );

    assert_dynamic_import(
      "
        async function test() {
          const {foo: bar} = await import('other');
          console.log(bar);
        }
     ",
      indoc! {r#"
        import "abc:other";
        async function $abc$var$test() {
            const { foo: bar } = await $abc$importAsync$70a00e0a8474f72a;
            console.log(bar);
        }
      "#},
      vec![foo_symbol(), foo_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(x => x.foo);
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then((x)=>x.foo);
      "#},
      vec![foo_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(x => x);
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then((x)=>x);
      "#},
      vec![star_symbol(), star_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(({foo}) => foo);
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then(({ foo: foo })=>foo);
      "#},
      vec![foo_symbol(), foo_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(({foo: bar}) => bar);
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then(({ foo: bar })=>bar);
      "#},
      vec![foo_symbol(), foo_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(function (x) { return x.foo });
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then(function(x) {
            return x.foo;
        });
      "#},
      vec![foo_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(function (x) { return x });
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then(function(x) {
            return x;
        });
      "#},
      vec![star_symbol(), star_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(function ({foo}) {});
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then(function({ foo: foo }) {});
      "#},
      vec![foo_symbol()],
    );

    assert_dynamic_import(
      "
        import('other').then(function ({foo: bar}) {});
     ",
      indoc! {r#"
        import "abc:other";
        $abc$importAsync$70a00e0a8474f72a.then(function({ foo: bar }) {});
      "#},
      vec![foo_symbol()],
    );

    fn foo_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("foo"),
        kind: ImportKind::DynamicImport,
        local: atom!("$abc$importAsync$70a00e0a8474f72a$6a5cdcad01c973fa"),
        source: atom!("other"),
      }
    }

    fn star_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("*"),
        kind: ImportKind::DynamicImport,
        local: atom!("$abc$importAsync$70a00e0a8474f72a"),
        source: atom!("other"),
      }
    }
  }

  #[test]
  fn test_requires() {
    fn assert_require(
      input_code: &str,
      expected_code: &str,
      imported_symbols: Vec<PartialImportedSymbol>,
    ) {
      let (code, hoist) = run_hoist(input_code);

      assert_eq!(code, expected_code);
      assert_eq!(hoist.re_exports, Vec::new());
      assert_eq!(
        map_imported_symbols(hoist.imported_symbols),
        imported_symbols
      );
    }

    assert_require(
      "require('other');",
      indoc! {r#"
        import "abc:other";
      "#},
      Vec::new(),
    );

    assert_require(
      "
        function x() {
          const foo = require('other');
          console.log(foo.bar);
        }
        require('bar');
      ",
      indoc! {r#"
        import "abc:other";
        function $abc$var$x() {
            const foo = $abc$import$70a00e0a8474f72a;
            console.log(foo.bar);
        }
        import "abc:bar";
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        function x() {
          console.log(require('other').foo);
        }
      ",
      indoc! {r#"
        import "abc:other";
        function $abc$var$x() {
            console.log($abc$import$70a00e0a8474f72a$6a5cdcad01c973fa);
        }
      "#},
      vec![foo_symbol()],
    );

    assert_require(
      "
        function x() {
          const {foo} = require('other');
          console.log(foo);
        }
      ",
      indoc! {r#"
        import "abc:other";
        function $abc$var$x() {
            const { foo: foo } = $abc$import$70a00e0a8474f72a;
            console.log(foo);
        }
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        function x() {
          const foo = require('other').foo;
          console.log(foo);
        }
      ",
      indoc! {r#"
        import "abc:other";
        function $abc$var$x() {
            const foo = $abc$import$70a00e0a8474f72a$6a5cdcad01c973fa;
            console.log(foo);
        }
      "#},
      vec![foo_symbol()],
    );

    assert_require(
      "
        function x() {
          const foo = require('other')[test];
          console.log(foo);
        }
      ",
      indoc! {r#"
        import "abc:other";
        function $abc$var$x() {
            const foo = $abc$import$70a00e0a8474f72a[test];
            console.log(foo);
        }
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        var foo = function () {
          if (Date.now() < 0) {
            var bar = require('other');
          }
        }();
      ",
      indoc! {r#"
        import "abc:other";
        var $abc$var$foo = function() {
            if (Date.now() < 0) {
                var bar = $abc$import$70a00e0a8474f72a;
            }
        }();
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        let x = require('a') + require('b');
      ",
      indoc! {r#"
        import "abc:a";
        import "abc:b";
        let $abc$var$x = $abc$import$407448d2b89b1813 + $abc$import$8b22cf2602fb60ce;
      "#},
      a_b_imported_symbols(),
    );

    assert_require(
      "
        let x = (require('a'), require('b'));
      ",
      indoc! {r#"
        import "abc:a";
        import "abc:b";
        let $abc$var$x = (!$abc$import$407448d2b89b1813, $abc$import$8b22cf2602fb60ce);
      "#},
      a_b_imported_symbols(),
    );

    assert_require(
      "
        let x = require('a') || require('b');
      ",
      indoc! {r#"
        import "abc:a";
        import "abc:b";
        let $abc$var$x = $abc$import$407448d2b89b1813 || $abc$import$8b22cf2602fb60ce;
      "#},
      a_b_imported_symbols(),
    );

    assert_require(
      "
        let x = condition ? require('a') : require('b');
      ",
      indoc! {r#"
        import "abc:a";
        import "abc:b";
        let $abc$var$x = condition ? $abc$import$407448d2b89b1813 : $abc$import$8b22cf2602fb60ce;
      "#},
      a_b_imported_symbols(),
    );

    assert_require(
      "
        if (condition) require('a');
      ",
      indoc! {r#"
        import "abc:a";
        if (condition) $abc$import$407448d2b89b1813;
      "#},
      vec![PartialImportedSymbol {
        imported: atom!("*"),
        kind: ImportKind::Require,
        local: atom!("$abc$import$407448d2b89b1813"),
        source: atom!("a"),
      }],
    );

    assert_require(
      "
        for (let x = require('y'); x < 5; x++) {}
      ",
      indoc! {r#"
        import "abc:y";
        for(let x = $abc$import$4a5767248b18ef41; x < 5; x++){}
      "#},
      vec![PartialImportedSymbol {
        imported: atom!("*"),
        kind: ImportKind::Require,
        local: atom!("$abc$import$4a5767248b18ef41"),
        source: atom!("y"),
      }],
    );

    assert_require(
      "
        const x = 4, {bar} = require('other'), baz = 3;
        console.log(bar);
      ",
      indoc! {r#"
        const $abc$var$x = 4;
        import "abc:other";
        var $abc$require$bar = $abc$import$70a00e0a8474f72a$d927737047eb3867;
        const $abc$var$baz = 3;
        console.log($abc$require$bar);
      "#},
      vec![PartialImportedSymbol {
        imported: atom!("bar"),
        kind: ImportKind::Require,
        local: atom!("$abc$import$70a00e0a8474f72a$d927737047eb3867"),
        source: atom!("other"),
      }],
    );

    assert_require(
      "
        const x = 3, foo = require('other'), bar = 2;
        console.log(foo.bar);
      ",
      indoc! {r#"
        const $abc$var$x = 3;
        import "abc:other";
        const $abc$var$bar = 2;
        console.log($abc$import$70a00e0a8474f72a$d927737047eb3867);
      "#},
      vec![PartialImportedSymbol {
        imported: atom!("bar"),
        kind: ImportKind::Require,
        local: atom!("$abc$import$70a00e0a8474f72a$d927737047eb3867"),
        source: atom!("other"),
      }],
    );

    assert_require(
      "
        const {foo, ...bar} = require('other');
        console.log(foo, bar);
      ",
      indoc! {r#"
        import "abc:other";
        const { foo: $abc$var$foo, ...$abc$var$bar } = $abc$import$70a00e0a8474f72a;
        console.log($abc$var$foo, $abc$var$bar);
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        const {x: {y: z}} = require('other');
        console.log(z);
      ",
      indoc! {r#"
        import "abc:other";
        const { x: { y: $abc$var$z } } = $abc$import$70a00e0a8474f72a;
        console.log($abc$var$z);
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        const foo = require('other');
        console.log(foo[bar]);
      ",
      indoc! {r#"
        import "abc:other";
        console.log($abc$import$70a00e0a8474f72a[bar]);
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        const foo = require('other');
        console.log(foo[bar], foo.baz);
      ",
      indoc! {r#"
        import "abc:other";
        console.log($abc$import$70a00e0a8474f72a[bar], $abc$import$70a00e0a8474f72a.baz);
      "#},
      vec![star_symbol(), star_symbol()],
    );

    assert_require(
      "
        const foo = require('other').foo;
        console.log(foo);
      ",
      indoc! {r#"
        import "abc:other";
        var $abc$require$foo = $abc$import$70a00e0a8474f72a$6a5cdcad01c973fa;
        console.log($abc$require$foo);
      "#},
      vec![foo_symbol()],
    );

    assert_require(
      "
        const foo = require('other')[bar];
        console.log(foo);
      ",
      indoc! {r#"
        import "abc:other";
        const $abc$var$foo = $abc$import$70a00e0a8474f72a[bar];
        console.log($abc$var$foo);
      "#},
      vec![star_symbol()],
    );

    assert_require(
      "
        const {foo} = require('other').foo;
        console.log(foo);
      ",
      indoc! {r#"
        import "abc:other";
        const { foo: $abc$var$foo } = $abc$import$70a00e0a8474f72a$6a5cdcad01c973fa;
        console.log($abc$var$foo);
      "#},
      vec![foo_symbol()],
    );

    fn foo_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("foo"),
        kind: ImportKind::Require,
        local: atom!("$abc$import$70a00e0a8474f72a$6a5cdcad01c973fa"),
        source: atom!("other"),
      }
    }

    fn star_symbol() -> PartialImportedSymbol {
      PartialImportedSymbol {
        imported: atom!("*"),
        kind: ImportKind::Require,
        local: atom!("$abc$import$70a00e0a8474f72a"),
        source: atom!("other"),
      }
    }

    fn a_b_imported_symbols() -> Vec<PartialImportedSymbol> {
      vec![
        PartialImportedSymbol {
          imported: atom!("*"),
          kind: ImportKind::Require,
          local: atom!("$abc$import$407448d2b89b1813"),
          source: atom!("a"),
        },
        PartialImportedSymbol {
          imported: atom!("*"),
          kind: ImportKind::Require,
          local: atom!("$abc$import$8b22cf2602fb60ce"),
          source: atom!("b"),
        },
      ]
    }
  }

  #[test]
  fn test_exports() {
    fn assert_exports(
      input_code: &str,
      expected_code: &str,
      exported_symbols: Vec<PartialExportedSymbol>,
    ) {
      let (code, hoist) = run_hoist(input_code);
      assert_eq!(code, expected_code);
      assert_eq!(hoist.self_references.len(), 0);
      assert_eq!(
        map_exported_symbols(hoist.exported_symbols),
        exported_symbols
      );
    }

    assert_exports(
      "
        let x = 1;
        let y = 2;
        let z = 3;
        export {x, y};
      ",
      indoc! {"
        let $abc$export$d141bba7fdc215a3 = 1;
        let $abc$export$4a5767248b18ef41 = 2;
        let $abc$var$z = 3;
      "},
      // TODO: There should be one less x and y symbol?
      vec![
        x_symbol(),
        y_symbol(),
        x_symbol(),
        x_symbol(),
        y_symbol(),
        y_symbol(),
      ],
    );

    assert_exports(
      "export default 1;",
      indoc! {"
        var $abc$export$2e2bcd8739ae039 = 1;
      "},
      vec![default_symbol()],
    );

    assert_exports(
      "
        let x = 3;
        export default x;
      ",
      indoc! {"
        let $abc$var$x = 3;
        var $abc$export$2e2bcd8739ae039 = $abc$var$x;
      "},
      vec![default_symbol()],
    );

    assert_exports(
      "export default function () {}",
      indoc! {"
        function $abc$export$2e2bcd8739ae039() {}
      "},
      vec![default_symbol()],
    );

    assert_exports(
      "export default class {}",
      indoc! {"
        class $abc$export$2e2bcd8739ae039 {
        }
      "},
      vec![default_symbol()],
    );

    assert_exports(
      "
        console.log(module);
        export default class X {}
      ",
      indoc! {"
        console.log(module);
        class X {
        }
      "},
      Vec::new(),
    );

    assert_exports(
      "export var x = 2, y = 3;",
      indoc! {"
        var $abc$export$d141bba7fdc215a3 = 2, $abc$export$4a5767248b18ef41 = 3;
      "},
      vec![x_symbol(), y_symbol()],
    );

    assert_exports(
      "
        export var {x, ...y} = something;
        export var [p, ...q] = something;
        export var {x = 3} = something;
      ",
      indoc! {"
        var { x: $abc$export$d141bba7fdc215a3, ...$abc$export$4a5767248b18ef41 } = something;
        var [$abc$export$ffb5f4729a158638, ...$abc$export$9e5f44173e64f162] = something;
        var { x: $abc$export$d141bba7fdc215a3 = 3 } = something;
      "},
      vec![
        PartialExportedSymbol {
          exported: atom!("x"),
          is_esm: true,
          local: atom!("$abc$export$d141bba7fdc215a3"),
        },
        PartialExportedSymbol {
          exported: atom!("y"),
          is_esm: true,
          local: atom!("$abc$export$4a5767248b18ef41"),
        },
        PartialExportedSymbol {
          exported: atom!("p"),
          is_esm: true,
          local: atom!("$abc$export$ffb5f4729a158638"),
        },
        PartialExportedSymbol {
          exported: atom!("q"),
          is_esm: true,
          local: atom!("$abc$export$9e5f44173e64f162"),
        },
        PartialExportedSymbol {
          exported: atom!("x"),
          is_esm: true,
          local: atom!("$abc$export$d141bba7fdc215a3"),
        },
      ],
    );

    assert_exports(
      "export function test() {}",
      indoc! {"
        function $abc$export$e0969da9b8fb378d() {}
      "},
      vec![PartialExportedSymbol {
        exported: atom!("test"),
        is_esm: true,
        local: atom!("$abc$export$e0969da9b8fb378d"),
      }],
    );

    assert_exports(
      "export class Test {}",
      indoc! {"
        class $abc$export$1b16fc9eb974a84d {
        }
      "},
      vec![PartialExportedSymbol {
        exported: atom!("Test"),
        is_esm: true,
        local: atom!("$abc$export$1b16fc9eb974a84d"),
      }],
    );

    assert_exports(
      "export {foo} from 'bar';",
      indoc! {r#"
        import "abc:bar:esm";
      "#},
      Vec::new(),
    );

    assert_exports(
      "export * from 'bar';",
      indoc! {r#"
        import "abc:bar:esm";
      "#},
      Vec::new(),
    );

    fn default_symbol() -> PartialExportedSymbol {
      PartialExportedSymbol {
        exported: atom!("default"),
        is_esm: true,
        local: atom!("$abc$export$2e2bcd8739ae039"),
      }
    }

    fn x_symbol() -> PartialExportedSymbol {
      PartialExportedSymbol {
        exported: atom!("x"),
        is_esm: true,
        local: atom!("$abc$export$d141bba7fdc215a3"),
      }
    }

    fn y_symbol() -> PartialExportedSymbol {
      PartialExportedSymbol {
        exported: atom!("y"),
        is_esm: true,
        local: atom!("$abc$export$4a5767248b18ef41"),
      }
    }
  }

  #[test]
  fn test_reexports() {
    fn assert_reexports(
      input_code: &str,
      expected_code: &str,
      exported_symbols: Vec<PartialExportedSymbol>,
      reexports: Vec<PartialImportedSymbol>,
    ) {
      let (code, hoist) = run_hoist(input_code);

      assert_eq!(code, expected_code);
      assert_eq!(hoist.self_references.len(), 0);
      assert_eq!(
        map_exported_symbols(hoist.exported_symbols),
        exported_symbols
      );

      assert_eq!(
        hoist
          .re_exports
          .into_iter()
          .map(PartialImportedSymbol::from)
          .collect::<Vec<PartialImportedSymbol>>(),
        reexports
      );
    }

    assert_reexports(
      "export { foo as bar } from './foo';",
      indoc! {r#"
        import "abc:./foo:esm";
      "#},
      vec![],
      vec![PartialImportedSymbol {
        imported: atom!("foo"),
        kind: ImportKind::Import,
        local: atom!("bar"),
        source: atom!("./foo"),
      }],
    );

    assert_reexports(
      "
        export { foo as bar } from './foo';
        export const foo = 1;
      ",
      indoc! {r#"
        import "abc:./foo:esm";
        const $abc$export$6a5cdcad01c973fa = 1;
      "#},
      vec![PartialExportedSymbol {
        exported: atom!("foo"),
        is_esm: true,
        local: atom!("$abc$export$6a5cdcad01c973fa"),
      }],
      vec![PartialImportedSymbol {
        imported: atom!("foo"),
        kind: ImportKind::Import,
        local: atom!("bar"),
        source: atom!("./foo"),
      }],
    );
  }

  #[test]
  fn test_cjs_exports() {
    fn assert_cjs_exports(
      input_code: &str,
      expected_code: &str,
      exported_symbols: Vec<PartialExportedSymbol>,
      self_references: HashSet<Atom>,
    ) {
      let (code, hoist) = run_hoist(input_code);

      assert_eq!(code, expected_code);
      assert_eq!(hoist.self_references, self_references);
      assert_eq!(
        map_exported_symbols(hoist.exported_symbols),
        exported_symbols
      );
    }

    for exports in ["exports", "module.exports"] {
      for input_code in [
        format!("{exports}.foo = 1;"),
        format!("{exports}['foo'] = 1;"),
      ] {
        assert_cjs_exports(
          &input_code,
          indoc! {"
            var $abc$export$6a5cdcad01c973fa;
            $abc$export$6a5cdcad01c973fa = 1;
          "},
          vec![foo_symbol()],
          HashSet::new(),
        );

        assert_cjs_exports(
          &formatdoc! {"
            {input_code}
            sideEffects(exports);
          "},
          &formatdoc! {"
            $abc${}
            sideEffects($abc$exports);
          ", input_code.replace("module.", "")},
          vec![star_symbol(), star_symbol()],
          HashSet::from([atom!("*")]),
        );
      }

      assert_cjs_exports(
        &formatdoc! {"
          {exports}.foo = 1;
          console.log({exports}.foo);
          sideEffects({exports}['foo']);
        "},
        indoc! {"
          var $abc$export$6a5cdcad01c973fa;
          $abc$export$6a5cdcad01c973fa = 1;
          console.log($abc$export$6a5cdcad01c973fa);
          sideEffects($abc$export$6a5cdcad01c973fa);
        "},
        vec![foo_symbol(), foo_symbol(), foo_symbol()],
        HashSet::from([atom!("foo")]),
      );

      assert_cjs_exports(
        &formatdoc! {"
          {exports}['foo'] = 1;
          console.log({exports}.foo);
          sideEffects({exports}['bar']);
        "},
        indoc! {"
          var $abc$export$6a5cdcad01c973fa;
          $abc$export$6a5cdcad01c973fa = 1;
          console.log($abc$export$6a5cdcad01c973fa);
          sideEffects($abc$export$d927737047eb3867);
        "},
        vec![foo_symbol(), foo_symbol(), bar_symbol()],
        HashSet::from([atom!("foo"), atom!("bar")]),
      );

      assert_cjs_exports(
        &format!("{exports}[foo] = 1;"),
        indoc! {"
          $abc$exports[foo] = 1;
        "},
        vec![star_symbol()],
        HashSet::new(),
      );

      assert_cjs_exports(
        &formatdoc! {"
          {exports}[foo] = 1;
          console.log({exports}.foo);
          console['log']({exports}['foo']);
          sideEffects({exports}[foo]);
        "},
        indoc! {"
          $abc$exports[foo] = 1;
          console.log($abc$exports.foo);
          console['log']($abc$exports['foo']);
          sideEffects($abc$exports[foo]);
        "},
        vec![star_symbol(), star_symbol(), star_symbol(), star_symbol()],
        HashSet::from([atom!("*")]),
      );

      assert_cjs_exports(
        &formatdoc! {"
          {exports}.foo = 1;
          {exports}[bar] = 2;
          {exports}['baz'] = 3;
        "},
        indoc! {"
          $abc$exports.foo = 1;
          $abc$exports[bar] = 2;
          $abc$exports['baz'] = 3;
        "},
        vec![star_symbol(), star_symbol(), star_symbol()],
        HashSet::new(),
      );

      assert_cjs_exports(
        &formatdoc! {"
          {exports}.foo = 1;
          sideEffects({});
        ", if exports == "exports" { "module.exports" } else { "exports" }},
        indoc! {"
          $abc$exports.foo = 1;
          sideEffects($abc$exports);
        "},
        vec![star_symbol(), star_symbol()],
        HashSet::from([atom!("*")]),
      );

      assert_cjs_exports(
        &formatdoc! {"
          {exports}.foo = 1;
          {exports}.bar = function() {{
            return {exports}.foo;
          }}
        "},
        indoc! {"
          var $abc$export$6a5cdcad01c973fa;
          var $abc$export$d927737047eb3867;
          $abc$export$6a5cdcad01c973fa = 1;
          $abc$export$d927737047eb3867 = function() {
              return $abc$export$6a5cdcad01c973fa;
          };
        "},
        vec![foo_symbol(), bar_symbol(), foo_symbol()],
        HashSet::from([atom!("foo")]),
      );

      assert_cjs_exports(
        &formatdoc! {"
          function main() {{
            {exports}.foo = 1;
          }}
        "},
        indoc! {"
          var $abc$export$6a5cdcad01c973fa;
          function $abc$var$main() {
              $abc$export$6a5cdcad01c973fa = 1;
          }
        "},
        vec![foo_symbol()],
        HashSet::new(),
      );

      if exports == "module.exports" {
        assert_cjs_exports(
          "
            var module = { exports: {} };
            module.exports.foo = 2;
            console.log(module.exports.foo);
          ",
          indoc! {"
            var $abc$var$module = {
                exports: {}
            };
            $abc$var$module.exports.foo = 2;
            console.log($abc$var$module.exports.foo);
          "},
          vec![],
          HashSet::new(),
        );
      }
    }

    fn foo_symbol() -> PartialExportedSymbol {
      PartialExportedSymbol {
        exported: atom!("foo"),
        is_esm: false,
        local: atom!("$abc$export$6a5cdcad01c973fa"),
      }
    }

    fn bar_symbol() -> PartialExportedSymbol {
      PartialExportedSymbol {
        exported: atom!("bar"),
        is_esm: false,
        local: atom!("$abc$export$d927737047eb3867"),
      }
    }

    fn star_symbol() -> PartialExportedSymbol {
      PartialExportedSymbol {
        exported: atom!("*"),
        is_esm: false,
        local: atom!("$abc$exports"),
      }
    }
  }

  #[test]
  fn test_this_assignment() {
    let (code, hoist) = run_hoist("this.foo = 1;");

    assert_eq!(
      code,
      indoc! {"
        var $abc$export$6a5cdcad01c973fa;
        $abc$export$6a5cdcad01c973fa = 1;
      "}
    );
    assert_eq!(hoist.self_references.len(), 0);
    assert_eq!(hoist.exported_symbols.len(), 1);
    assert_eq!(
      hoist.exported_symbols[0].local,
      atom!("$abc$export$6a5cdcad01c973fa")
    );
    assert_eq!(hoist.exported_symbols[0].exported, atom!("foo"));
  }

  #[test]
  fn test_vars() {
    let (code, _hoist) = run_hoist(
      "
        var x = 2;
        var y = {x};
        var z = {x: 3};
        var w = {[x]: 4};

        function test() {
          var x = 3;
        }
      ",
    );
    assert_eq!(
      code,
      indoc! {"
        var $abc$var$x = 2;
        var $abc$var$y = {
            x: $abc$var$x
        };
        var $abc$var$z = {
            x: 3
        };
        var $abc$var$w = {
            [$abc$var$x]: 4
        };
        function $abc$var$test() {
            var x = 3;
        }
      "}
    );
  }

  #[derive(Debug, Eq, Hash, PartialEq)]
  struct PartialImportedSymbol {
    imported: Atom,
    kind: ImportKind,
    local: Atom,
    source: Atom,
  }

  impl From<ImportedSymbol> for PartialImportedSymbol {
    fn from(symbol: ImportedSymbol) -> Self {
      PartialImportedSymbol {
        imported: symbol.imported,
        kind: symbol.kind,
        local: symbol.local,
        source: symbol.source,
      }
    }
  }

  #[derive(Debug, Eq, Hash, PartialEq)]
  struct PartialExportedSymbol {
    exported: Atom,
    is_esm: bool,
    local: Atom,
  }

  impl From<ExportedSymbol> for PartialExportedSymbol {
    fn from(symbol: ExportedSymbol) -> Self {
      PartialExportedSymbol {
        exported: symbol.exported,
        is_esm: symbol.is_esm,
        local: symbol.local,
      }
    }
  }

  fn run_hoist(input_code: &str) -> (String, HoistResult) {
    let RunVisitResult {
      output_code: collect_output_code,
      visitor: collect,
      ..
    } = run_test_visit_const(input_code, |context| {
      Collect::new(
        SymbolsInfo::default(),
        context.source_map.clone(),
        context.unresolved_mark,
        Mark::fresh(Mark::root()),
        context.global_mark,
        true,
        context.is_module,
        false,
      )
    });

    let RunVisitResult {
      output_code,
      visitor: hoist,
      ..
    } = run_test_fold(&collect_output_code, |context| {
      Hoist::new("abc", context.unresolved_mark, &collect)
    });

    (output_code, hoist.get_result())
  }

  fn map_exported_symbols(exported_symbols: Vec<ExportedSymbol>) -> Vec<PartialExportedSymbol> {
    exported_symbols
      .into_iter()
      .map(PartialExportedSymbol::from)
      .collect::<Vec<PartialExportedSymbol>>()
  }

  fn map_imported_symbols(imported_symbols: Vec<ImportedSymbol>) -> Vec<PartialImportedSymbol> {
    imported_symbols
      .into_iter()
      .map(PartialImportedSymbol::from)
      .collect::<Vec<PartialImportedSymbol>>()
  }
}
