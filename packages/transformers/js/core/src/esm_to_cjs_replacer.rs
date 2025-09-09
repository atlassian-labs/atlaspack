use std::collections::HashMap;
use std::collections::HashSet;

use inflector::Inflector;
use swc_core::common::Mark;
use swc_core::common::Span;
use swc_core::common::SyntaxContext;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::js_word;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::preset_env::Feature;
use swc_core::ecma::preset_env::Versions;
use swc_core::ecma::visit::VisitMut;
use swc_core::ecma::visit::VisitMutWith;

use crate::id;
use crate::utils::get_undefined_ident;
use crate::utils::match_export_name;
use crate::utils::match_export_name_ident;

pub struct EsmToCjsReplacer {
  // Map of imported identifier to (source, specifier)
  imports: HashMap<Id, (JsWord, JsWord)>,
  // Map of source to (require identifier, mark)
  require_names: HashMap<JsWord, (JsWord, Mark)>,
  // Set of declared default interops, by source.
  interops: HashSet<JsWord>,
  // List of requires to insert at the top of the module.
  requires: Vec<ModuleItem>,
  // List of exports to add.
  exports: Vec<ModuleItem>,
  pub needs_helpers: bool,
  in_export_decl: bool,
  in_function_scope: bool,
  mark: Mark,
  unresolved_mark: Mark,
  versions: Option<Versions>,
}

fn local_name_for_src(src: &JsWord) -> JsWord {
  if !src.contains('/') {
    return format!("_{}", src.to_camel_case()).into();
  }

  format!("_{}", src.split('/').last().unwrap().to_camel_case()).into()
}

impl EsmToCjsReplacer {
  pub fn new(unresolved_mark: Mark, versions: Option<Versions>) -> Self {
    EsmToCjsReplacer {
      imports: HashMap::new(),
      require_names: HashMap::new(),
      interops: HashSet::new(),
      requires: vec![],
      exports: vec![],
      needs_helpers: false,
      in_export_decl: false,
      in_function_scope: false,
      mark: Mark::fresh(Mark::root()),
      unresolved_mark,
      versions,
    }
  }

  fn get_require_name(&mut self, src: &JsWord, span: Span) -> Ident {
    if let Some((name, mark)) = self.require_names.get(src) {
      return Ident::new(name.clone(), span, SyntaxContext::empty().apply_mark(*mark));
    }

    let name = local_name_for_src(src);
    let mark = Mark::fresh(Mark::root());
    self.require_names.insert(src.clone(), (name.clone(), mark));
    Ident::new(name, span, SyntaxContext::empty().apply_mark(mark))
  }

  fn get_interop_default_name(&mut self, src: &JsWord) -> Ident {
    self.get_require_name(src, DUMMY_SP);
    let (name, mark) = self.require_names.get(src).unwrap();
    Ident::new(
      format!("{}Default", name).into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(*mark),
    )
  }

  fn create_require(&mut self, src: JsWord, span: Span) {
    if self.require_names.contains_key(&src) {
      return;
    }

    let ident = self.get_require_name(&src, DUMMY_SP);
    let require = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
      span,
      ctxt: SyntaxContext::empty(),
      kind: VarDeclKind::Var,
      decls: vec![VarDeclarator {
        span: DUMMY_SP,
        name: Pat::Ident(ident.into()),
        init: Some(Box::new(Expr::Call(crate::utils::create_require(
          src,
          self.unresolved_mark,
        )))),
        definite: false,
      }],
      declare: false,
    }))));

    self.requires.push(require)
  }

  fn create_interop_default(&mut self, src: JsWord) {
    if self.interops.contains(&src) {
      return;
    }

    let local = self.get_require_name(&src, DUMMY_SP);
    let ident = self.get_interop_default_name(&src);
    let interop = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      kind: VarDeclKind::Var,
      decls: vec![VarDeclarator {
        span: DUMMY_SP,
        name: Pat::Ident(ident.into()),
        init: Some(Box::new(self.create_helper_call(
          "interopDefault".into(),
          vec![Expr::Ident(local)],
          DUMMY_SP,
        ))),
        definite: false,
      }],
      declare: false,
    }))));

    self.requires.push(interop);
    self.interops.insert(src);
  }

  fn create_helper_call(&mut self, name: JsWord, args: Vec<Expr>, span: Span) -> Expr {
    self.needs_helpers = true;
    let ident = Ident::new(
      "parcelHelpers".into(),
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(self.mark),
    );
    Expr::Call(CallExpr {
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        obj: Box::new(Expr::Ident(ident)),
        prop: MemberProp::Ident(IdentName::new(name, DUMMY_SP)),
        span: DUMMY_SP,
      }))),
      args: args
        .iter()
        .map(|arg| ExprOrSpread {
          expr: Box::new(arg.clone()),
          spread: None,
        })
        .collect(),
      span,
      ctxt: SyntaxContext::empty(),
      type_args: None,
    })
  }

  fn call_helper(&mut self, name: JsWord, args: Vec<Expr>, span: Span) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      expr: Box::new(self.create_helper_call(name, args, span)),
      span,
    }))
  }

  fn create_export(&mut self, exported: JsWord, local: Expr, span: Span) {
    let export = self.call_helper(
      js_word!("export"),
      vec![
        Expr::Ident(Ident::new_no_ctxt("exports".into(), DUMMY_SP)),
        Expr::Lit(Lit::Str(exported.into())),
        if matches!(self.versions, Some(versions) if Feature::ArrowFunctions.should_enable(versions, true, false)) {
          Expr::Fn(FnExpr {
            ident: None,
            function: Box::new(Function {
              body: Some(BlockStmt {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                stmts: vec![Stmt::Return({
                  ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(local)),
                  }
                })],
              }),
              is_async: false,
              is_generator: false,
              params: vec![],
              decorators: vec![],
              span: DUMMY_SP,
              ctxt: SyntaxContext::empty(),
              return_type: None,
              type_params: None,
            }),
          })
        } else {
          Expr::Arrow(ArrowExpr {
            body: Box::new(BlockStmtOrExpr::Expr(Box::new(local))),
            is_async: false,
            is_generator: false,
            params: vec![],
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            return_type: None,
            type_params: None,
          })
        },
      ],
      span,
    );
    self.exports.push(export)
  }

  fn create_exports_assign(&mut self, name: JsWord, right: Expr, span: Span) -> ModuleItem {
    ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      expr: Box::new(Expr::Assign(AssignExpr {
        op: AssignOp::Assign,
        left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
          obj: Box::new(Expr::Ident(Ident::new_no_ctxt("exports".into(), DUMMY_SP))),
          prop: MemberProp::Ident(IdentName::new(name, DUMMY_SP)),
          span: DUMMY_SP,
        })),
        right: Box::new(right),
        span: DUMMY_SP,
      })),
      span,
    }))
  }

  fn create_import_access(&mut self, source: &JsWord, imported: &JsWord, span: Span) -> Expr {
    if imported == "*" {
      let name = self.get_require_name(source, span);
      return Expr::Ident(name);
    }

    let obj = if imported == "default" {
      self.get_interop_default_name(source)
    } else {
      self.get_require_name(source, DUMMY_SP)
    };

    // import { foo } from "..."; foo();
    // ->
    // import { foo } from "..."; (0, foo)();
    Expr::Seq(SeqExpr {
      exprs: vec![
        0.into(),
        Box::new(Expr::Member(MemberExpr {
          obj: Box::new(Expr::Ident(obj)),
          prop: MemberProp::Ident(IdentName::new(imported.clone(), DUMMY_SP)),
          span,
        })),
      ],
      span,
    })
  }

  fn extract_exports_from_decl(&mut self, var: &VarDecl) -> Vec<ModuleItem> {
    let mut exports = vec![];

    for decl in &var.decls {
      match &decl.name {
        Pat::Ident(binding_ident) => {
          self.extract_exports_from_binding_ident(binding_ident, &mut exports)
        }
        Pat::Array(array_pat) => self.extract_exports_from_array_pattern(array_pat, &mut exports),
        Pat::Rest(rest_pat) => self.extract_exports_from_rest_pattern(rest_pat, &mut exports),
        Pat::Object(object_pat) => {
          self.extract_exports_from_object_pattern(object_pat, &mut exports);
        }
        Pat::Assign(assign_pat) => {
          self.extract_exports_from_assign_pattern(assign_pat, &mut exports)
        }
        Pat::Invalid(_) => {}
        Pat::Expr(_) => {}
      }
    }

    exports
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
  fn extract_exports_from_binding_ident(
    &mut self,
    binding_ident: &BindingIdent,
    exports: &mut Vec<ModuleItem>,
  ) {
    let ident = binding_ident.id.clone();
    let export =
      self.create_exports_assign(ident.sym.clone(), Expr::Ident(ident.clone()), DUMMY_SP);
    exports.push(export);
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
  fn extract_exports_from_object_pattern(
    &mut self,
    object_pat: &ObjectPat,
    exports: &mut Vec<ModuleItem>,
  ) {
    for prop in &object_pat.props {
      match prop {
        // This is `foo` in:
        // { foo }
        ObjectPatProp::Assign(prop) => {
          let key = prop.key.clone();
          assert!(prop.value.is_none());
          let export =
            self.create_exports_assign(key.sym.clone(), Expr::Ident(key.id.clone()), DUMMY_SP);
          exports.push(export);
        }
        // This is `foo` in:
        // { prop: foo }
        ObjectPatProp::KeyValue(prop) => {
          let value = &prop.value;
          self.extract_exports_from_pat(&*value, exports);
        }
        // This is `foo` in:
        // { ...foo }
        ObjectPatProp::Rest(rest_pat) => {
          self.extract_exports_from_rest_pattern(rest_pat, exports);
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
  fn extract_exports_from_pat(&mut self, pat: &Pat, exports: &mut Vec<ModuleItem>) {
    match pat {
      Pat::Ident(binding_ident) => {
        self.extract_exports_from_binding_ident(binding_ident, exports);
      }
      Pat::Array(array_pat) => {
        self.extract_exports_from_array_pattern(array_pat, exports);
      }
      Pat::Object(object_pat) => {
        self.extract_exports_from_object_pattern(object_pat, exports);
      }
      Pat::Rest(rest_pat) => {
        self.extract_exports_from_rest_pattern(rest_pat, exports);
      }
      Pat::Assign(assign_pat) => {
        self.extract_exports_from_assign_pattern(assign_pat, exports);
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
  fn extract_exports_from_array_pattern(
    &mut self,
    array_pat: &ArrayPat,
    exports: &mut Vec<ModuleItem>,
  ) {
    for elem in &array_pat.elems {
      if let Some(elem) = elem {
        self.extract_exports_from_pat(elem, exports);
      }
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
  fn extract_exports_from_rest_pattern(
    &mut self,
    rest_pat: &RestPat,
    exports: &mut Vec<ModuleItem>,
  ) {
    self.extract_exports_from_pat(&*rest_pat.arg, exports);
  }

  /// Extracts exports from an assign pattern.
  ///
  /// This happens when we have:
  ///
  ///     export const { a = 10 } = ...;
  ///     export const { <PAT> = <DEFAULT> } = ...;
  ///
  /// We recursively extract exports from `<PAT>`.
  fn extract_exports_from_assign_pattern(
    &mut self,
    assign_pat: &AssignPat,
    exports: &mut Vec<ModuleItem>,
  ) {
    self.extract_exports_from_pat(&*assign_pat.left, exports);
  }
}

macro_rules! visit_function_scope {
  ($name:ident, $type:ident) => {
    fn $name(&mut self, node: &mut $type) {
      let in_function_scope = self.in_function_scope;
      self.in_function_scope = true;
      node.visit_mut_children_with(self);
      self.in_function_scope = in_function_scope;
    }
  };
}

impl VisitMut for EsmToCjsReplacer {
  fn visit_mut_module(&mut self, node: &mut Module) {
    let mut is_esm = false;
    let mut needs_interop_flag = false;

    // First pass: collect all imported declarations. On the second pass, exports can be matched to
    // imports (to better handle import/export pairs that are really just reexports).
    //
    // To ensure that all declarations that cause dependencies are kept in the same order, handle
    // export declarations with a source in the first pass as well.
    for item in &node.body {
      if let ModuleItem::ModuleDecl(decl) = &item {
        is_esm = true;
        match decl {
          ModuleDecl::Import(import) => {
            self.create_require(import.src.value.clone(), import.span);

            for specifier in &import.specifiers {
              match specifier {
                ImportSpecifier::Named(named) => {
                  let imported = match &named.imported {
                    Some(imported) => match_export_name(imported).0.clone(),
                    None => named.local.sym.clone(),
                  };
                  self.imports.insert(
                    id!(named.local),
                    (import.src.value.clone(), imported.clone()),
                  );
                  if imported == js_word!("default") {
                    self.create_interop_default(import.src.value.clone());
                  }
                }
                ImportSpecifier::Default(default) => {
                  self.imports.insert(
                    id!(default.local),
                    (import.src.value.clone(), "default".into()),
                  );
                  self.create_interop_default(import.src.value.clone());
                }
                ImportSpecifier::Namespace(namespace) => {
                  self
                    .imports
                    .insert(id!(namespace.local), (import.src.value.clone(), "*".into()));
                }
              }
            }
          }
          ModuleDecl::ExportNamed(export) => {
            needs_interop_flag = true;

            if let Some(src) = &export.src {
              self.create_require(src.value.clone(), export.span);

              for specifier in &export.specifiers {
                match specifier {
                  ExportSpecifier::Named(named) => {
                    let exported = match &named.exported {
                      Some(exported) => exported.clone(),
                      None => named.orig.clone(),
                    };

                    if match_export_name(&named.orig).0 == js_word!("default") {
                      self.create_interop_default(src.value.clone());
                    }

                    let specifier = self.create_import_access(
                      &src.value,
                      &match_export_name(&named.orig).0,
                      DUMMY_SP,
                    );
                    self.create_export(match_export_name(&exported).0, specifier, export.span);
                  }
                  ExportSpecifier::Default(default) => {
                    self.create_interop_default(src.value.clone());
                    let specifier =
                      self.create_import_access(&src.value, &js_word!("default"), DUMMY_SP);
                    self.create_export(default.exported.sym.clone(), specifier, export.span);
                  }
                  ExportSpecifier::Namespace(namespace) => {
                    let local = self.get_require_name(&src.value, DUMMY_SP);
                    self.create_export(
                      match_export_name(&namespace.name).0,
                      Expr::Ident(local),
                      export.span,
                    )
                  }
                }
              }
            } else {
              // Handled below
            }
          }
          ModuleDecl::ExportAll(export) => {
            needs_interop_flag = true;
            self.create_require(export.src.value.clone(), export.span);
            let require_name = self.get_require_name(&export.src.value, export.span);
            let export = self.call_helper(
              "exportAll".into(),
              vec![
                Expr::Ident(require_name),
                Expr::Ident(Ident::new_no_ctxt("exports".into(), DUMMY_SP)),
              ],
              export.span,
            );
            self.requires.push(export);
          }
          _ => (),
        }
      }
    }

    // If we didn't see any module declarations, nothing to do.
    if !is_esm {
      return;
    }

    node.visit_mut_children_with(self);
    let mut items = vec![];

    // Second pass
    for item in &node.body {
      match &item {
        ModuleItem::ModuleDecl(decl) => {
          match decl {
            ModuleDecl::Import(_) | ModuleDecl::ExportAll(_) => {
              // Handled above
            }
            ModuleDecl::ExportNamed(export) => {
              needs_interop_flag = true;
              if export.src.is_none() {
                for specifier in &export.specifiers {
                  if let ExportSpecifier::Named(named) = specifier {
                    let exported = match &named.exported {
                      Some(exported) => exported.clone(),
                      None => named.orig.clone(),
                    };
                    let orig = match_export_name_ident(&named.orig);

                    // Handle import {foo} from 'bar'; export {foo};
                    let value =
                      if let Some((source, imported)) = self.imports.get(&id!(orig)).cloned() {
                        self.create_import_access(
                          &source,
                          &imported,
                          match_export_name(&named.orig).1,
                        )
                      } else {
                        Expr::Ident(orig.clone())
                      };

                    self.create_export(match_export_name(&exported).0, value, export.span);
                  }
                }
              } else {
                // Handled above
              }
            }
            ModuleDecl::ExportDefaultExpr(export) => {
              needs_interop_flag = true;
              items.push(self.create_exports_assign(
                "default".into(),
                *export.expr.clone(),
                export.span,
              ))
            }
            ModuleDecl::ExportDefaultDecl(export) => {
              needs_interop_flag = true;

              match &export.decl {
                DefaultDecl::Class(class) => {
                  if let Some(ident) = &class.ident {
                    items.push(ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
                      ident: ident.clone(),
                      declare: false,
                      class: class.class.clone(),
                    }))));
                    items.push(self.create_exports_assign(
                      "default".into(),
                      Expr::Ident(ident.clone()),
                      DUMMY_SP,
                    ));
                  } else {
                    items.push(self.create_exports_assign(
                      "default".into(),
                      Expr::Class(ClassExpr {
                        ident: None,
                        class: class.class.clone(),
                      }),
                      export.span,
                    ));
                  }
                }
                DefaultDecl::Fn(func) => {
                  if let Some(ident) = &func.ident {
                    items.push(ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
                      ident: ident.clone(),
                      declare: false,
                      function: func.function.clone(),
                    }))));
                    self.create_export("default".into(), Expr::Ident(ident.clone()), DUMMY_SP);
                  } else {
                    self.create_export(
                      "default".into(),
                      Expr::Fn(FnExpr {
                        ident: None,
                        function: func.function.clone(),
                      }),
                      export.span,
                    );
                  }
                }
                _ => {
                  unreachable!("unsupported export default declaration");
                }
              }
            }
            ModuleDecl::ExportDecl(export) => {
              match &export.decl {
                Decl::Class(class) => {
                  self.create_export(
                    class.ident.sym.clone(),
                    Expr::Ident(class.ident.clone()),
                    export.span,
                  );

                  let mut decl = export.decl.clone();
                  decl.visit_mut_with(self);

                  items.push(ModuleItem::Stmt(Stmt::Decl(decl)));
                }
                Decl::Fn(func) => {
                  self.create_export(
                    func.ident.sym.clone(),
                    Expr::Ident(func.ident.clone()),
                    export.span,
                  );

                  let mut decl = export.decl.clone();
                  decl.visit_mut_with(self);

                  items.push(ModuleItem::Stmt(Stmt::Decl(decl)));
                }
                Decl::Var(var) => {
                  let mut var = var.clone();

                  if var.kind == VarDeclKind::Const {
                    let exports = self.extract_exports_from_decl(&var);
                    items.push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))));
                    items.extend(exports);

                    continue;
                  }

                  var.decls = var
                    .decls
                    .iter()
                    .map(|decl| {
                      let mut decl = decl.clone();
                      self.in_export_decl = true;
                      decl.name.visit_mut_with(self);
                      self.in_export_decl = false;
                      decl.init.visit_mut_with(self);
                      decl
                    })
                    .collect();

                  items.push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))));
                }
                _ => {
                  let mut decl = export.decl.clone();
                  decl.visit_mut_with(self);

                  items.push(ModuleItem::Stmt(Stmt::Decl(decl)));
                }
              }

              needs_interop_flag = true;
            }
            _ => items.push(item.clone()),
          }
        }
        _ => items.push(item.clone()),
      }
    }

    if needs_interop_flag {
      let helper = self.call_helper(
        "defineInteropFlag".into(),
        vec![Expr::Ident(Ident::new_no_ctxt("exports".into(), DUMMY_SP))],
        DUMMY_SP,
      );
      self.exports.insert(0, helper);
    }

    items.splice(0..0, self.requires.clone());
    items.splice(0..0, self.exports.clone());

    if self.needs_helpers {
      items.insert(
        0,
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          kind: VarDeclKind::Var,
          decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(
              Ident::new(
                "parcelHelpers".into(),
                DUMMY_SP,
                SyntaxContext::empty().apply_mark(self.mark),
              )
              .into(),
            ),
            init: Some(Box::new(Expr::Call(crate::utils::create_require(
              "@atlaspack/transformer-js/src/esmodule-helpers.js".into(),
              self.unresolved_mark,
            )))),
            definite: false,
          }],
          declare: false,
        })))),
      )
    }

    node.body = items;
  }

  fn visit_mut_binding_ident(&mut self, node: &mut BindingIdent) {
    if self.in_export_decl {
      // export const {foo} = ...;
      // println!("exporting {:?}", node.id.sym);
      self.create_export(node.id.sym.clone(), Expr::Ident(node.id.clone()), DUMMY_SP);
    }

    node.visit_mut_children_with(self);
  }

  visit_function_scope!(visit_mut_function, Function);
  visit_function_scope!(visit_mut_class, Class);
  visit_function_scope!(visit_mut_getter_prop, GetterProp);
  visit_function_scope!(visit_mut_setter_prop, SetterProp);

  fn visit_mut_expr(&mut self, node: &mut Expr) {
    match &node {
      Expr::Ident(ident) => {
        if let Some((source, imported)) = self.imports.get(&id!(ident)).cloned() {
          *node = self.create_import_access(&source, &imported, ident.span);
        }
      }
      Expr::This(_this) => {
        if !self.in_function_scope {
          *node = Expr::Ident(get_undefined_ident(self.unresolved_mark));
        }
      }
      _ => {
        node.visit_mut_children_with(self);
      }
    }
  }

  fn visit_mut_prop(&mut self, node: &mut Prop) {
    // let obj = {a, b}; -> let obj = {a: imported.a, b: imported.b};
    if let Some(ident) = node.as_mut_shorthand() {
      if let Some((source, imported)) = self.imports.get(&id!(ident)).cloned() {
        *node = Prop::KeyValue(KeyValueProp {
          key: PropName::Ident(IdentName::new(ident.sym.clone(), DUMMY_SP)),
          value: Box::new(self.create_import_access(&source, &imported, ident.span)),
        });

        return;
      }
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
    node.obj.visit_mut_with(self);

    if let MemberProp::Computed(_) = node.prop {
      node.prop.visit_mut_with(self);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use atlaspack_swc_runner::test_utils::{run_test_visit, RunVisitResult};
  use indoc::indoc;
  use swc_core::ecma::preset_env::{BrowserData, Version};

  use super::*;

  #[test]
  fn transforms_imports_to_cjs() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        import { useEffect } from 'react';

        useEffect(() => {});
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var _react = require("react");
        0, _react.useEffect(()=>{});
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn transforms_imports_and_object_expressions_referencing_import_specifiers_to_cjs() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        import { a, b } from 'foo';

        const obj = { a, b };
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var _foo = require("foo");
        const obj = {
            a: 0, _foo.a,
            b: 0, _foo.b
        };
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn transforms_imports_and_computed_object_keys_referencing_import_specifiers_to_cjs() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        import { a, b } from 'foo';

        const obj = { [a]: 1, [b]: 2 };
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var _foo = require("foo");
        const obj = {
            [0, _foo.a]: 1,
            [0, _foo.b]: 2
        };
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn transforms_imports_and_object_values_referencing_import_specifiers_to_cjs() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        import { a, b } from 'foo';

        const obj = { hello: a, world: b };
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var _foo = require("foo");
        const obj = {
            hello: 0, _foo.a,
            world: 0, _foo.b
        };
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn transforms_imports_and_computed_member_expressions_referencing_import_specifiers_to_cjs() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        import { foo } from 'foo';

        const obj = foo[bar]();
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var _foo = require("foo");
        const obj = 0, _foo.foo[bar]();
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn transforms_export_all_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export * from './main';
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
        parcelHelpers.defineInteropFlag(exports);
        var _main = require("./main");
        parcelHelpers.exportAll(_main, exports);
      "#}
    );

    assert!(visitor.needs_helpers);
  }

  #[test]
  fn transforms_default_export_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export default function main() {}
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
        parcelHelpers.defineInteropFlag(exports);
        parcelHelpers.export(exports, "default", ()=>main);
        function main() {}
      "#}
    );

    assert!(visitor.needs_helpers);
  }

  #[test]
  fn transforms_named_export_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export function main() {}
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
        parcelHelpers.defineInteropFlag(exports);
        parcelHelpers.export(exports, "main", ()=>main);
        function main() {}
      "#}
    );

    assert!(visitor.needs_helpers);
  }

  // Unneeded on spec and not how Chrome works
  #[test]
  fn does_not_transform_destructured_object_export_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const { main } = obj;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { main } = obj;
        exports.main = main;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn does_not_transform_destructured_object_export_with_rename_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const { main: foo } = obj;
        export const x = foo;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { main: foo } = obj;
        exports.foo = foo;
        const x = foo;
        exports.x = x;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn does_not_transform_destructured_object_export_with_array_destructuring_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const { main: [foo] } = obj;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { main: [foo] } = obj;
        exports.foo = foo;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn does_not_transform_destructured_object_with_nested_destructuring_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const { main: { foo } } = obj;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { main: { foo } } = obj;
        exports.foo = foo;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn test_extracts_rest_patterns_in_objects() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const { main: foo, ...rest } = obj;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { main: foo, ...rest } = obj;
        exports.foo = foo;
        exports.rest = rest;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn test_extracts_assign_patterns_in_objects() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const { main: foo = 1 } = obj;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const { main: foo = 1 } = obj;
        exports.foo = foo;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn does_not_transform_constant_bindings_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const something = 10;
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const something = 10;
        exports.something = something;
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn does_not_transform_module_exports_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        module.exports = function main1() {}
        module.exports.main = function main2() {}
        exports = function main3() {}
        exports.main = function main4() {}
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        module.exports = function main1() {};
        module.exports.main = function main2() {};
        exports = function main3() {};
        exports.main = function main4() {};
      "#}
    );

    assert!(!visitor.needs_helpers);
  }

  #[test]
  fn transforms_imports_and_exports_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        import { useEffect } from 'react';
        export function main() {
          useEffect(() => {}, []);
        }
      "#,
      |context| EsmToCjsReplacer::new(context.unresolved_mark, None),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
        parcelHelpers.defineInteropFlag(exports);
        parcelHelpers.export(exports, "main", ()=>main);
        var _react = require("react");
        function main() {
            0, _react.useEffect(()=>{}, []);
        }
      "#}
    );

    assert!(visitor.needs_helpers);
  }

  #[test]
  fn does_not_transforms_arrow_functions_to_use_helpers() {
    let RunVisitResult {
      output_code,
      visitor,
      ..
    } = run_test_visit(
      r#"
        export const main1 = () => {};
        const main2 = () => {};
      "#,
      |context| {
        EsmToCjsReplacer::new(
          context.unresolved_mark,
          Some(BrowserData {
            chrome: Some(Version::from_str("1.0.0").unwrap()),
            ..BrowserData::default()
          }),
        )
      },
    );

    // TODO: Should main1 and main2 not include an arrow function?
    assert_eq!(
      output_code,
      indoc! {r#"
        const main1 = ()=>{};
        exports.main1 = main1;
        const main2 = ()=>{};
      "#}
    );

    assert!(!visitor.needs_helpers);
  }
}
