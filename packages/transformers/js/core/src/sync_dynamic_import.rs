use serde::{Deserialize, Serialize};
use std::path::Path;
use swc_core::common::Mark;
use swc_core::common::SyntaxContext;
use swc_core::ecma::{
  ast::*,
  visit::{VisitMut, VisitMutWith},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct SyncDynamicImportConfig {
  pub entrypoint_filepath_suffix: String,
  pub actual_require_paths: Vec<String>,
}

pub struct SyncDynamicImport<'a> {
  filename: &'a Path,
  unresolved_mark: Mark,
  config: &'a Option<SyncDynamicImportConfig>,
}

impl<'a> SyncDynamicImport<'a> {
  pub fn new(
    filename: &'a Path,
    unresolved_mark: Mark,
    config: &'a Option<SyncDynamicImportConfig>,
  ) -> Self {
    Self {
      filename,
      unresolved_mark,
      config,
    }
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
    // (()=>{
    //     const fileExports = require(SOURCE);
    //     return Promise.resolve(fileExports);
    // })()
    let require_call = self.create_require_call(source_args);

    Expr::Call(CallExpr {
      span: Default::default(),
      callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
        span: Default::default(),
        expr: Box::new(Expr::Arrow(ArrowExpr {
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
        })),
      }))),
      args: vec![],
      type_args: None,
      ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
    })
  }

  fn should_convert_to_require(&self, module_path: &str) -> bool {
    self
      .config
      .as_ref()
      .map(|config| {
        config
          .actual_require_paths
          .iter()
          .any(|path| module_path.contains(path))
      })
      .unwrap_or(false)
  }

  fn is_entrypoint_file(&self) -> bool {
    self
      .filename
      .to_str()
      .zip(self.config.as_ref())
      .map(|(f, config)| f.ends_with(&config.entrypoint_filepath_suffix))
      .unwrap_or(false)
  }

  // Simple path join function
  // does not support resolution of inbetween back paths
  // i.e /root/../src
  fn resolve_module_path(&self, source: &str) -> String {
    let mut head_path: &Path = self.filename;
    let mut destination: &str = source;

    if source.starts_with("./") {
      destination = source.trim_start_matches("./");
    } else if source.starts_with("../") {
      for _i in 0..source.matches("../").count() {
        if let Some(parent_path) = head_path.parent() {
          head_path = parent_path;

          if head_path == Path::new("/") {
            break;
          }
        }
      }

      destination = source.trim_start_matches("../");
    } else if source.starts_with("/") {
      head_path = Path::new("/");
      destination = source.trim_start_matches("/");
    }

    head_path.join(Path::new(destination)).display().to_string()
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
            if self.should_convert_to_require(&self.resolve_module_path(&path)) {
              *expr = self.create_require_call(call.args.clone());

              return;
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

  fn get_config() -> Option<SyncDynamicImportConfig> {
    Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![
        "src/packages/router-resources".into(),
        "@atlaskit/tokens".into(),
      ],
    })
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
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &None,
        )
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
    let config = get_config();
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('./src/packages/router-resources/module.tsx');
        export function foo() {
          const moduleWithOpts = import(/* webpackChunkName: "tokens" */'@atlaskit/tokens', { with: 'json' });
          const dummy = import('./dummy');
        }
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
        )
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const module = require('./src/packages/router-resources/module.tsx');
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
    let config = get_config();
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
          Path::new("/repo/entrypoint.tsx"),
          run_test_context.unresolved_mark,
          &config,
        )
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export const modalEntryPoint = createEntryPoint({
            root: JSResourceForInteraction(()=>(()=>{
                    const fileExports = require('./modal.tsx');
                    return Promise.resolve(fileExports);
                })())
        });
      "#}
    );
  }

  #[test]
  fn test_entrypoint_file_with_chained_promise() {
    let config = get_config();
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        export const modalEntryPoint = createEntryPoint({
          root: JSResourceForInteraction(() =>
            import(
              /* webpackChunkName: "async-modal-entrypoint" */ './modal.tsx'
            ).then((module) => module.modal),
          ),
        });
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/entrypoint.tsx"),
          run_test_context.unresolved_mark,
          &config,
        )
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export const modalEntryPoint = createEntryPoint({
            root: JSResourceForInteraction(()=>(()=>{
                    const fileExports = require('./modal.tsx');
                    return Promise.resolve(fileExports);
                })().then((module)=>module.modal))
        });
      "#}
    );
  }

  #[test]
  fn test_resolve_current_directory_source() {
    let instance = SyncDynamicImport::new(Path::new("/repo/index.tsx"), Mark::root(), &None);

    let single_pattern_result = instance.resolve_module_path("./src/packages/index.tsx");
    let multi_pattern_result = instance.resolve_module_path("./././src/packages/index.tsx");

    assert_eq!(
      single_pattern_result,
      "/repo/index.tsx/src/packages/index.tsx"
    );

    assert_eq!(
      multi_pattern_result,
      "/repo/index.tsx/src/packages/index.tsx"
    );
  }

  #[test]
  fn test_resolve_back_directory_source() {
    let instance =
      SyncDynamicImport::new(Path::new("/repo/product/index.tsx"), Mark::root(), &None);

    let single_pattern_result = instance.resolve_module_path("../src/packages/index.tsx");
    let multi_pattern_result = instance.resolve_module_path("../../src/packages/index.tsx");
    let back_to_root_result = instance.resolve_module_path("../../../src/packages/index.tsx");
    let passed_root_result = instance.resolve_module_path("../../../../src/packages/index.tsx");

    assert_eq!(
      single_pattern_result,
      "/repo/product/src/packages/index.tsx"
    );
    assert_eq!(multi_pattern_result, "/repo/src/packages/index.tsx");
    assert_eq!(back_to_root_result, "/src/packages/index.tsx");
    assert_eq!(passed_root_result, "/src/packages/index.tsx");
  }

  #[test]
  fn test_resolve_root_source() {
    let instance = SyncDynamicImport::new(Path::new("/repo/index.tsx"), Mark::root(), &None);

    let result = instance.resolve_module_path("/src/packages/index.tsx");

    assert_eq!(result, "/src/packages/index.tsx");
  }

  #[test]
  // Update test if functionality is required
  fn test_resolve_not_supported_source() {
    let instance = SyncDynamicImport::new(Path::new("/repo/index.tsx"), Mark::root(), &None);

    let result = instance.resolve_module_path("./src/../packages/index.tsx");

    // To match Node's Path.resolve the result should be
    // "/repo/product/index.tsx/packages/index.tsx"
    assert_eq!(result, "/repo/index.tsx/src/../packages/index.tsx");
  }
}
