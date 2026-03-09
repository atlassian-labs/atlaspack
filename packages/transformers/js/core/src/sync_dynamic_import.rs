use serde::{Deserialize, Serialize};
use std::path::Path;
use swc_core::common::Mark;
use swc_core::common::SyntaxContext;
use swc_core::ecma::{
  ast::*,
  visit::{VisitMut, VisitMutWith},
};

#[derive(Clone, Debug, Deserialize, Serialize, Hash)]
pub struct SyncDynamicImportConfig {
  pub entrypoint_filepath_suffix: String,
  pub actual_require_paths: Vec<String>,
  #[serde(default)]
  pub sync_require_paths: Vec<String>,
}

pub struct SyncDynamicImport<'a> {
  filename: &'a Path,
  unresolved_mark: Mark,
  config: &'a Option<SyncDynamicImportConfig>,
  reject_with_error: bool,
}

impl<'a> SyncDynamicImport<'a> {
  pub fn new(
    filename: &'a Path,
    unresolved_mark: Mark,
    config: &'a Option<SyncDynamicImportConfig>,
    reject_with_error: bool,
  ) -> Self {
    Self {
      filename,
      unresolved_mark,
      config,
      reject_with_error,
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

  fn create_unresolved_import_promise(&self, import_path: &Option<String>) -> Expr {
    if self.reject_with_error {
      self.create_rejecting_promise(import_path)
    } else {
      self.create_rejecting_promise_old(import_path)
    }
  }

  /// Legacy rejecting promise that rejects with a plain string.
  /// Kept for backwards compatibility behind the `syncDynamicImportRejectWithError` feature flag.
  fn create_rejecting_promise_old(&self, import_path: &Option<String>) -> Expr {
    // new Promise((_resolve, reject) => {
    //   if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT) {
    //     reject('...')
    //   }
    // })
    let path_str = import_path
      .as_ref()
      .map(|p| p.as_str())
      .unwrap_or("unknown");
    let escaped_path = path_str.replace('\'', "\\'");
    let error_message = format!(
      "A dynamic import() statement to path \"{}\" was used in SSR code, but only synchronous (require()) imports will work. To include this code in the SSR bundle, update the `actual_require_paths` property of SYNC_DYNAMIC_IMPORT_CONFIG",
      escaped_path
    );

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
          params: vec![
            Pat::Ident(BindingIdent {
              id: Ident::new(
                "_resolve".into(),
                Default::default(),
                SyntaxContext::empty().apply_mark(self.unresolved_mark),
              ),
              type_ann: None,
            }),
            Pat::Ident(BindingIdent {
              id: Ident::new(
                "reject".into(),
                Default::default(),
                SyntaxContext::empty().apply_mark(self.unresolved_mark),
              ),
              type_ann: None,
            }),
          ],
          body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
            span: Default::default(),
            stmts: vec![Stmt::If(IfStmt {
              span: Default::default(),
              test: Box::new(Expr::Member(MemberExpr {
                span: Default::default(),
                obj: Box::new(Expr::Ident(Ident::new(
                  "globalThis".into(),
                  Default::default(),
                  SyntaxContext::empty().apply_mark(self.unresolved_mark),
                ))),
                prop: MemberProp::Ident("__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT".into()),
              })),
              cons: Box::new(Stmt::Block(BlockStmt {
                span: Default::default(),
                stmts: vec![Stmt::Expr(ExprStmt {
                  span: Default::default(),
                  expr: Box::new(Expr::Call(CallExpr {
                    span: Default::default(),
                    callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                      "reject".into(),
                      Default::default(),
                      SyntaxContext::empty().apply_mark(self.unresolved_mark),
                    )))),
                    args: vec![ExprOrSpread {
                      spread: None,
                      expr: Box::new(Expr::Lit(Lit::Str(Str {
                        span: Default::default(),
                        value: error_message.into(),
                        raw: None,
                      }))),
                    }],
                    type_args: None,
                    ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  })),
                })],
                ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
              })),
              alt: None,
            })],
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

  /// Rejecting promise that rejects with `new Error(message)` and attaches `.skipSsr = true`.
  /// Enabled by the `syncDynamicImportRejectWithError` feature flag.
  fn create_rejecting_promise(&self, import_path: &Option<String>) -> Expr {
    // new Promise((_resolve, reject) => {
    //   if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT) {
    //     const error = new Error('...');
    //     error.skipSsr = true;
    //     reject(error);
    //   }
    // })
    let path_str = import_path
      .as_ref()
      .map(|p| p.as_str())
      .unwrap_or("unknown");
    let escaped_path = path_str.replace('\'', "\\'");
    let error_message = format!(
      "A dynamic import() statement to path \"{}\" was used in SSR code, but only synchronous (require()) imports will work. To include this code in the SSR bundle, update the `actual_require_paths` property of SYNC_DYNAMIC_IMPORT_CONFIG",
      escaped_path
    );

    let promise_body = Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
      span: Default::default(),
      stmts: vec![Stmt::If(IfStmt {
        span: Default::default(),
        test: Box::new(Expr::Member(MemberExpr {
          span: Default::default(),
          obj: Box::new(Expr::Ident(Ident::new(
            "globalThis".into(),
            Default::default(),
            SyntaxContext::empty().apply_mark(self.unresolved_mark),
          ))),
          prop: MemberProp::Ident("__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT".into()),
        })),
        cons: Box::new(Stmt::Block(BlockStmt {
          span: Default::default(),
          stmts: vec![
            // const error = new Error('...');
            Stmt::Decl(Decl::Var(Box::new(VarDecl {
              span: Default::default(),
              ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
              kind: VarDeclKind::Const,
              declare: false,
              decls: vec![VarDeclarator {
                span: Default::default(),
                name: Pat::Ident(BindingIdent {
                  id: Ident::new(
                    "error".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ),
                  type_ann: None,
                }),
                init: Some(Box::new(Expr::New(NewExpr {
                  span: Default::default(),
                  callee: Box::new(Expr::Ident(Ident::new(
                    "Error".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ))),
                  args: Some(vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(Str {
                      span: Default::default(),
                      value: error_message.into(),
                      raw: None,
                    }))),
                  }]),
                  type_args: None,
                  ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
                }))),
                definite: false,
              }],
            }))),
            // error.skipSsr = true;
            Stmt::Expr(ExprStmt {
              span: Default::default(),
              expr: Box::new(Expr::Assign(AssignExpr {
                span: Default::default(),
                op: AssignOp::Assign,
                left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
                  span: Default::default(),
                  obj: Box::new(Expr::Ident(Ident::new(
                    "error".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ))),
                  prop: MemberProp::Ident("skipSsr".into()),
                })),
                right: Box::new(Expr::Lit(Lit::Bool(Bool {
                  span: Default::default(),
                  value: true,
                }))),
              })),
            }),
            // reject(error);
            Stmt::Expr(ExprStmt {
              span: Default::default(),
              expr: Box::new(Expr::Call(CallExpr {
                span: Default::default(),
                callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                  "reject".into(),
                  Default::default(),
                  SyntaxContext::empty().apply_mark(self.unresolved_mark),
                )))),
                args: vec![ExprOrSpread {
                  spread: None,
                  expr: Box::new(Expr::Ident(Ident::new(
                    "error".into(),
                    Default::default(),
                    SyntaxContext::empty().apply_mark(self.unresolved_mark),
                  ))),
                }],
                type_args: None,
                ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
              })),
            }),
          ],
          ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
        })),
        alt: None,
      })],
      ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
    }));
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
          params: vec![
            Pat::Ident(BindingIdent {
              id: Ident::new(
                "_resolve".into(),
                Default::default(),
                SyntaxContext::empty().apply_mark(self.unresolved_mark),
              ),
              type_ann: None,
            }),
            Pat::Ident(BindingIdent {
              id: Ident::new(
                "reject".into(),
                Default::default(),
                SyntaxContext::empty().apply_mark(self.unresolved_mark),
              ),
              type_ann: None,
            }),
          ],
          body: promise_body,
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

  fn create_promise_resolved_require(&self, args: Vec<ExprOrSpread>) -> Expr {
    // Promise.resolve(require(SOURCE))
    let require_call = self.create_require_call(args);

    Expr::Call(CallExpr {
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
        expr: Box::new(require_call),
      }],
      type_args: None,
      ctxt: SyntaxContext::empty().apply_mark(self.unresolved_mark),
    })
  }

  fn should_convert_to_sync_require(&self, module_path: &str) -> bool {
    self
      .config
      .as_ref()
      .map(|config| {
        config
          .sync_require_paths
          .iter()
          .any(|path| module_path.contains(path))
      })
      .unwrap_or(false)
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
    if let Expr::Call(call) = expr
      && let Callee::Import(_) = &call.callee
      && let Some(first_arg) = call.args.first()
    {
      let is_string_literal = matches!(&*first_arg.expr, Expr::Lit(Lit::Str(_)) | Expr::Tpl(_));

      if !is_string_literal {
        // Replace with rejecting promise for non-string imports
        *expr = self.create_unresolved_import_promise(&None);
        return;
      }

      // Extract the import path for string literals
      let import_path = match &*first_arg.expr {
        Expr::Lit(Lit::Str(str_lit)) => Some(str_lit.value.to_string()),
        _ => None,
      };

      if let Some(path) = &import_path {
        // Check if this is an entrypoint file
        if self.is_entrypoint_file() {
          let entrypoint_expr = self.create_entrypoint_import(call.args.clone());
          *expr = entrypoint_expr;
          return;
        }

        let resolved_path = self.resolve_module_path(path);

        // Check sync_require_paths first (substring matching, wraps in Promise.resolve)
        if self.should_convert_to_sync_require(&resolved_path) {
          *expr = self.create_promise_resolved_require(call.args.clone());
          return;
        }

        // Check actual_require_paths (substring matching, bare require)
        if self.should_convert_to_require(&resolved_path) {
          *expr = self.create_require_call(call.args.clone());
          return;
        }

        // Default case: replace with rejecting promise using resolved path
        *expr = self.create_unresolved_import_promise(&Some(resolved_path));
      } else {
        // No path extracted, use None
        *expr = self.create_unresolved_import_promise(&None);
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
  use pretty_assertions::assert_eq;

  fn get_config() -> Option<SyncDynamicImportConfig> {
    Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![
        "src/packages/router-resources".into(),
        "@atlaskit/tokens".into(),
      ],
      sync_require_paths: vec![],
    })
  }

  #[test]
  fn test_rejecting_promise_without_config() {
    // When no config is provided, should generate rejecting promise (old style with string)
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
          false,
        )
      },
    );

    // All imports should generate rejecting promises (no config means no require conversion)
    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("reject('A dynamic import()"));
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
          false,
        )
      },
    );

    // Matched paths become require(), unmatched become rejecting promises
    assert!(output_code.contains("require('./src/packages/router-resources/module.tsx')"));
    assert!(output_code.contains("require('@atlaskit/tokens'"));
    assert!(output_code.contains("reject('A dynamic import()"));
    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
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
          false,
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
          false,
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
    let instance = SyncDynamicImport::new(Path::new("/repo/index.tsx"), Mark::root(), &None, false);

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
    let instance = SyncDynamicImport::new(
      Path::new("/repo/product/index.tsx"),
      Mark::root(),
      &None,
      false,
    );

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
    let instance = SyncDynamicImport::new(Path::new("/repo/index.tsx"), Mark::root(), &None, false);

    let result = instance.resolve_module_path("/src/packages/index.tsx");

    assert_eq!(result, "/src/packages/index.tsx");
  }

  #[test]
  // Update test if functionality is required
  fn test_resolve_not_supported_source() {
    let instance = SyncDynamicImport::new(Path::new("/repo/index.tsx"), Mark::root(), &None, false);

    let result = instance.resolve_module_path("./src/../packages/index.tsx");

    // To match Node's Path.resolve the result should be
    // "/repo/product/index.tsx/packages/index.tsx"
    assert_eq!(result, "/repo/index.tsx/src/../packages/index.tsx");
  }

  #[test]
  fn test_rejecting_promise_runtime_behavior() {
    // This test documents the runtime behavior of the rejecting promise
    // When globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT is true, the promise should reject
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec![],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const moduleA = import('unmatched-module-a');
        const moduleB = import('unmatched-module-b');
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    // Verify that unmatched imports generate promises with rejection logic
    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("reject('A dynamic import()"));
    assert!(output_code.contains("was used in SSR code"));
  }

  #[test]
  fn test_rejecting_promise_with_single_quote_in_path() {
    // Test that single quotes in the module path are properly escaped
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec![],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import("./it's-a-module");
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    // Verify the single quote is escaped in the output (with triple backslash as SWC escapes it)
    assert!(output_code.contains(r"it\\\'s-a-module"));
    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("A dynamic import() statement to path"));
  }

  #[test]
  fn test_rejecting_promise_for_unmatched_paths() {
    // Unmatched paths should always generate rejecting promises
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec![],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('module');
        const dummy = import('./dummy');
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("reject('A dynamic import()"));
    assert!(output_code.contains("/repo/index.tsx/module"));
    assert!(output_code.contains("/repo/index.tsx/dummy"));
  }

  #[test]
  fn test_sync_require_paths_with_substring_matching() {
    // sync_require_paths should use substring matching and wrap in Promise.resolve(require(...))
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec!["router-resources".into(), "@atlaskit/tokens".into()],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('./src/packages/router-resources/module.tsx');
        export function foo() {
          const tokens = import('@atlaskit/tokens');
          const dummy = import('./dummy');
        }
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    // Matched paths should become Promise.resolve(require(...))
    assert!(
      output_code
        .contains("Promise.resolve(require('./src/packages/router-resources/module.tsx'))")
    );
    assert!(output_code.contains("Promise.resolve(require('@atlaskit/tokens'))"));
    // Unmatched path should become rejecting promise
    assert!(output_code.contains("reject('A dynamic import()"));
  }

  #[test]
  fn test_sync_require_paths_takes_priority_over_actual_require_paths() {
    // sync_require_paths should take priority over actual_require_paths
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec!["@atlaskit/tokens".into()],
      sync_require_paths: vec!["@atlaskit/tokens".into()],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const tokens = import('@atlaskit/tokens');
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    // sync_require_paths should win, producing Promise.resolve(require(...))
    assert!(output_code.contains("Promise.resolve(require('@atlaskit/tokens'))"));
    // Should NOT be a bare require (which actual_require_paths would produce)
    assert!(!output_code.starts_with("const tokens = require("));
  }

  #[test]
  fn test_sync_require_paths_substring_matching() {
    // Test substring matching for sync_require_paths
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec!["components".into()],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const a = import('./src/packages/components/Button.tsx');
        const b = import('./src/utils/helper.tsx');
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    // Matching path should become Promise.resolve(require(...))
    assert!(
      output_code.contains("Promise.resolve(require('./src/packages/components/Button.tsx'))")
    );
    // Non-matching path should become rejecting promise
    assert!(output_code.contains("reject('A dynamic import()"));
  }

  #[test]
  fn test_rejecting_promise_with_error_flag_enabled() {
    // When syncDynamicImportRejectWithError is true, should reject with new Error() + skipSsr
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec![],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('module');
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          true,
        )
      },
    );

    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("const error = new Error("));
    assert!(output_code.contains("error.skipSsr = true"));
    assert!(output_code.contains("reject(error)"));
    // Should NOT contain the old-style plain string reject
    assert!(!output_code.contains("reject('A dynamic import()"));
  }

  #[test]
  fn test_rejecting_promise_old_style_with_flag_disabled() {
    // When syncDynamicImportRejectWithError is false, should reject with plain string (old behaviour)
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      sync_require_paths: vec![],
    });

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const module = import('module');
      "#},
      |run_test_context: RunTestContext| {
        SyncDynamicImport::new(
          Path::new("/repo/index.tsx"),
          run_test_context.unresolved_mark,
          &config,
          false,
        )
      },
    );

    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("reject('A dynamic import()"));
    // Should NOT contain Error object or skipSsr
    assert!(!output_code.contains("new Error("));
    assert!(!output_code.contains("skipSsr"));
  }
}
