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
              needs_interop_flag = true;
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

  #[test]
  fn transforms_destructured_object_export_to_use_helpers() {
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
        var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
        parcelHelpers.defineInteropFlag(exports);
        parcelHelpers.export(exports, "main", ()=>main);
        const { main } = obj;
      "#}
    );

    assert!(visitor.needs_helpers);
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
  fn transforms_arrow_functions_to_use_helpers_when_unsupported() {
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
        var parcelHelpers = require("@atlaspack/transformer-js/src/esmodule-helpers.js");
        parcelHelpers.defineInteropFlag(exports);
        parcelHelpers.export(exports, "main1", function() {
            return main1;
        });
        const main1 = ()=>{};
        const main2 = ()=>{};
      "#}
    );

    assert!(visitor.needs_helpers);
  }
}
