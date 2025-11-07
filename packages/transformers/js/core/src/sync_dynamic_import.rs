use std::path::Path;
use swc_core::common::Mark;
use swc_core::common::SyntaxContext;
use swc_core::ecma::{
  ast::*,
  visit::{VisitMut, VisitMutWith},
};

pub struct SyncDynamicImport<'a> {
  filename: &'a Path,
  unresolved_mark: Mark,
  resolve_module_path_override: Option<fn(&str) -> Option<String>>,
}

#[allow(dead_code)]
impl<'a> SyncDynamicImport<'a> {
  pub fn new(filename: &'a Path, unresolved_mark: Mark) -> Self {
    Self {
      filename,
      unresolved_mark,
      resolve_module_path_override: None,
    }
  }

  fn __override_resolve_module_path(&mut self, override_fn: Option<fn(&str) -> Option<String>>) {
    self.resolve_module_path_override = override_fn;
  }

  fn create_require_call(&self, args: Vec<ExprOrSpread>) -> Expr {
    Expr::Call(CallExpr {
      span: Default::default(),
      callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
        "require".into(),
        Default::default(),
        SyntaxContext::empty().apply_mark(self.unresolved_mark),
      )))),
      args,
      type_args: None,
      ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
    })
  }

  fn create_dummy_promise(&self) -> Expr {
    // new Promise(() => {})
    Expr::New(NewExpr {
      span: Default::default(),
      callee: Box::new(Expr::Ident(Ident::new(
        "Promise".into(),
        Default::default(),
        SyntaxContext::empty().apply_mark(self.unresolved_mark),
      ))),
      args: Some(vec![ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Arrow(ArrowExpr {
          span: Default::default(),
          params: vec![],
          body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: Default::default(),
            stmts: vec![],
            ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
          })),
          is_async: false,
          is_generator: false,
          type_params: None,
          return_type: None,
          ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
        })),
      }]),
      type_args: None,
      ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
    })
  }

  fn create_entrypoint_import(&self, source_args: Vec<ExprOrSpread>) -> Expr {
    // {
    //     const fileExports = require(SOURCE);
    //     return Promise.resolve(fileExports);
    // }
    let require_call = self.create_require_call(source_args);

    Expr::Call(CallExpr {
      span: Default::default(),
      callee: Callee::Expr(Box::new(Expr::Arrow(ArrowExpr {
        span: Default::default(),
        params: vec![],
        body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
          span: Default::default(),
          ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
          stmts: vec![
            // const fileExports = require(SOURCE);
            Stmt::Decl(Decl::Var(Box::new(VarDecl {
              span: Default::default(),
              ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
              kind: VarDeclKind::Const,
              declare: false,
              decls: vec![VarDeclarator {
                span: Default::default(),
                name: Pat::Ident(BindingIdent {
                  id: Ident::new(
                    "fileExports".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ),
                  type_ann: None,
                }),
                init: Some(Box::new(require_call)),
                definite: false,
              }],
            }))),
            // return Promise.resolve(fileExports);
            Stmt::Return(ReturnStmt {
              span: Default::default(),
              arg: Some(Box::new(Expr::Call(CallExpr {
                span: Default::default(),
                callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                  span: Default::default(),
                  obj: Box::new(Expr::Ident(Ident::new(
                    "Promise".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ))),
                  prop: MemberProp::Ident("resolve".into()),
                }))),
                args: vec![ExprOrSpread {
                  spread: None,
                  expr: Box::new(Expr::Ident(Ident::new(
                    "fileExports".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ))),
                }],
                type_args: None,
                ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
              }))),
            }),
          ],
        })),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
        ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
      }))),
      args: vec![],
      type_args: None,
      ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
    })
  }

  fn should_convert_to_require(&self, module_path: &str) -> bool {
    let allowed_paths = vec![
      "src/packages/router-resources",
      "@atlaskit/tokens",
      "platform/packages/design-system/tokens",
    ];

    allowed_paths.iter().any(|&path| module_path.contains(path))
  }

  fn is_entrypoint_file(&self) -> bool {
    self
      .filename
      .to_str()
      .map(|f| f.ends_with("entrypoint.tsx"))
      .unwrap_or(false)
  }

  fn resolve_module_path(&self, import_path: &str) -> Option<String> {
    if let Some(override_fn) = self.resolve_module_path_override {
      override_fn(import_path)
    } else {
      if let Ok(resolved) = self.filename.parent()?.join(import_path).canonicalize() {
        return resolved.to_str().map(|s| s.to_string());
      }

      return None;
    }
  }
}

impl<'a> VisitMut for SyncDynamicImport<'a> {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    // Check if this is a dynamic import call
    if let Expr::Call(call) = expr {
      if let Callee::Import(_) = &call.callee {
        if let Some(first_arg) = call.args.first() {
          let is_string_literal = matches!(&*first_arg.expr, Expr::Lit(Lit::Str(_)) | Expr::Tpl(_));

          if !is_string_literal {
            // Replace with dummy promise for non-string imports
            *expr = self.create_dummy_promise();
            return;
          }

          // Extract the import path for string literals
          let import_path = match &*first_arg.expr {
            Expr::Lit(Lit::Str(str_lit)) => Some(str_lit.value.to_string()),
            _ => None,
          };

          if let Some(path) = import_path {
            // Check if this is an entrypoint file
            if self.is_entrypoint_file() {
              let entrypoint_expr = self.create_entrypoint_import(call.args.clone());
              *expr = entrypoint_expr;
              return;
            }

            // Check if we should convert to require based on module path
            if let Some(resolved_path) = self.resolve_module_path(&path) {
              if self.should_convert_to_require(&resolved_path) {
                *expr = self.create_require_call(call.args.clone());

                return;
              }
            }
          }

          // Default case: replace with dummy promise
          *expr = self.create_dummy_promise();
        }
      }
    }

    // Continue visiting child nodes
    expr.visit_mut_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  fn mock_resolve(file_path: &str) -> Option<String> {
    return Some(file_path.to_string());
  }

  #[test]
  fn test_dummy_resolve() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('module');
        export const bar = () => import('bar').then(m => m.Bar);
        export function foo() {
          import('foo');
        }
        const path = '../path/index.tsx';
        const dummy = import(path);
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(Path::new("./index.tsx"), run_test_context.unresolved_mark)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const module = new Promise(()=>{});
        export const bar = ()=>new Promise(()=>{}).then((m)=>m.Bar);
        export function foo() {
            new Promise(()=>{});
        }
        const path = '../path/index.tsx';
        const dummy = new Promise(()=>{});
      "#}
    );
  }

  #[test]
  fn test_require_replacement() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('src/packages/router-resources/module.tsx');
        export function foo() {
          const moduleWithOpts = import(/* webpackChunkName: "tokens" */'@atlaskit/tokens', { with: 'json' });
          const dummy = import('./dummy');
        }
      "#},
      |run_test_context: RunTestContext| {
        let mut transformer =
          SyncDynamicImport::new(Path::new("./index.tsx"), run_test_context.unresolved_mark);

        transformer.__override_resolve_module_path(Some(mock_resolve));
        transformer
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const module = require('src/packages/router-resources/module.tsx');
        export function foo() {
            const moduleWithOpts = require('@atlaskit/tokens', {
                with: 'json'
            });
            const dummy = new Promise(()=>{});
        }
      "#}
    );
  }

  #[test]
  fn test_entrypoint_file() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        export const modalEntryPoint = createEntryPoint({
          root: JSResourceForInteraction(() =>
            import(
              /* webpackChunkName: "async-modal-entrypoint" */ './modal.tsx'
            ),
          ),
        });
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("./entrypoint.tsx"),
          run_test_context.unresolved_mark,
        )
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export const modalEntryPoint = createEntryPoint({
            root: JSResourceForInteraction(()=>()=>{
                    const fileExports = require('./modal.tsx');
                    return Promise.resolve(fileExports);
                }())
        });
      "#}
    );
  }
}
