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
  pub activate_reject_on_unresolved_imports: bool,
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

  fn create_unresolved_import_promise(&self, import_path: &Option<String>) -> Expr {
    // Check if we should activate rejecting promises based on config
    let should_reject = self
      .config
      .as_ref()
      .map(|config| config.activate_reject_on_unresolved_imports)
      .unwrap_or(false);

    if should_reject {
      self.create_rejecting_promise(import_path)
    } else {
      self.create_dummy_promise()
    }
  }

  fn create_rejecting_promise(&self, import_path: &Option<String>) -> Expr {
    // new Promise((_resolve, reject) => {
    //   if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT) {
    //     reject('...')
    //   }
    // })
    let path_str = import_path
      .as_ref()
      .map(|p| p.as_str())
      .unwrap_or("unknown");
    // Escape single quotes in the path for the JavaScript string literal
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
      .map(|(f, config)| {
        f.to_lowercase()
          .ends_with(&config.entrypoint_filepath_suffix.to_lowercase())
      })
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
        // Replace with rejecting or dummy promise for non-string imports
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
        // Check if we should convert to require based on module path
        if self.should_convert_to_require(&resolved_path) {
          *expr = self.create_require_call(call.args.clone());

          return;
        }

        // Default case: replace with rejecting or dummy promise using resolved path
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

  fn get_config() -> Option<SyncDynamicImportConfig> {
    Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![
        "src/packages/router-resources".into(),
        "@atlaskit/tokens".into(),
      ],
      activate_reject_on_unresolved_imports: false,
    })
  }

  #[test]
  fn test_dummy_promise_without_config() {
    // When no config is provided, should generate dummy promise (default behavior)
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
  fn test_entrypoint_file_case_insensitive() {
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
          Path::new("/repo/modalEntrypoint.tsx"),
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

  #[test]
  fn test_rejecting_promise_runtime_behavior() {
    // This test documents the runtime behavior of the rejecting promise
    // When globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT is true, the promise should reject
    // This is a documentation test showing what the generated code should do
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      activate_reject_on_unresolved_imports: true,
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
        )
      },
    );

    // Verify that unmatched imports generate promises with rejection logic
    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("reject('A dynamic import() statement to path"));
    assert!(output_code.contains("was used in SSR code"));
  }

  #[test]
  fn test_rejecting_promise_with_single_quote_in_path() {
    // Test that single quotes in the module path are properly escaped when rejecting is enabled
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      activate_reject_on_unresolved_imports: true,
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
        )
      },
    );

    // Verify the single quote is escaped in the output (with triple backslash as SWC escapes it)
    assert!(output_code.contains(r"it\\\'s-a-module"));
    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("A dynamic import() statement to path"));
  }

  #[test]
  fn test_dummy_promise_when_config_disabled() {
    // When activate_reject_on_unresolved_imports is false, should generate dummy promise
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      activate_reject_on_unresolved_imports: false,
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
        )
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const module = new Promise(()=>{});
        const dummy = new Promise(()=>{});
      "#}
    );
  }

  #[test]
  fn test_rejecting_promise_when_config_enabled() {
    // When activate_reject_on_unresolved_imports is true, should generate rejecting promise
    let config = Some(SyncDynamicImportConfig {
      entrypoint_filepath_suffix: "entrypoint.tsx".into(),
      actual_require_paths: vec![],
      activate_reject_on_unresolved_imports: true,
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
        )
      },
    );

    assert!(output_code.contains("if (globalThis.__SSR_TEMP_THROW_ON_UNRESOLVED_DYNAMIC_IMPORT)"));
    assert!(output_code.contains("reject('A dynamic import() statement to path"));
    assert!(output_code.contains("/repo/index.tsx/module"));
    assert!(output_code.contains("/repo/index.tsx/dummy"));
  }
}
