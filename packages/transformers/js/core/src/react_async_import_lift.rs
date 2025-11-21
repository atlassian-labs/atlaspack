use std::collections::HashSet;

use swc_core::common::{DUMMY_SP, Mark, Span, SyntaxContext};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_core::quote;

/// Transformer that lifts dynamic imports out of JSResourceForUserVisible calls
/// from @atlassian/react-async for SSR optimization.
///
/// This transform enables SSR optimization by hoisting `import()` calls inside
/// JSResourceForUserVisible loader functions to the module scope as `require()` calls.
/// The lifting behavior is controlled by the `entryFsSsrLiftImportToModule` option
/// in the options object passed to JSResourceForUserVisible.
///
/// # Example
///
/// Input:
/// ```js
/// import { JSResourceForUserVisible } from '@atlassian/react-async';
/// export const MyEntryPoint = createEntryPoint({
///   root: JSResourceForUserVisible(
///     () => import('./src/ui/index.tsx'),
///     { moduleId: "abc123", entryFsSsrLiftImportToModule: true },
///   ),
/// });
/// ```
///
/// Output:
/// ```js
/// const _liftedReactAsyncImport = function () {
///   {
///     const fileExports = _interopRequireWildcard(require('./src/ui/index.tsx'));
///     return Promise.resolve(fileExports);
///   }
/// }();
/// import { JSResourceForUserVisible } from '@atlassian/react-async';
/// export const MyEntryPoint = createEntryPoint({
///   root: JSResourceForUserVisible(() => _liftedReactAsyncImport, {
///     moduleId: "abc123",
///     entryFsSsrLiftImportToModule: true
///   }),
/// });
/// ```
pub struct ReactAsyncImportLift {
  /// Mark used to identify unresolved (global/imported) references
  pub unresolved_mark: Mark,
  /// Whether import lifting is applied regardless of import options
  pub import_lifting_by_default: bool,
  pub reporting_level: String,
  /// Collected lifted imports to be inserted at module scope
  pub lifted_imports: Vec<VarDeclarator>,
  /// Counter for generating unique lifted import identifiers
  pub lift_counter: usize,
  /// Set to track bindings of JSResourceForUserVisible imports
  pub jsx_resource_bindings: HashSet<Id>,
}

impl ReactAsyncImportLift {
  pub fn new(
    unresolved_mark: Mark,
    import_lifting_by_default: bool,
    reporting_level: String,
  ) -> Self {
    Self {
      unresolved_mark,
      import_lifting_by_default,
      reporting_level,
      lifted_imports: Vec::new(),
      lift_counter: 0,
      jsx_resource_bindings: HashSet::new(),
    }
  }

  /// Check if import lifting is enabled for a given options object
  fn is_import_lifted(&self, opts: &ObjectLit) -> bool {
    // Look for entryFsSsrLiftImportToModule property
    for prop in &opts.props {
      if let PropOrSpread::Prop(prop) = prop
        && let Prop::KeyValue(kv) = &**prop
        && let PropName::Ident(ident) = &kv.key
        && &*ident.sym == "entryFsSsrLiftImportToModule"
        && let Expr::Lit(Lit::Bool(bool_lit)) = &*kv.value
      {
        return bool_lit.value;
      }
    }

    match self.reporting_level.as_str() {
      "report" => println!("No entryFsSsrLiftImportToModule setting found"),
      "error" => eprintln!("No entryFsSsrLiftImportToModule setting found"),
      _ => {}
    }

    self.import_lifting_by_default
  }

  /// Generate a unique identifier for a lifted import
  fn generate_lifted_import_id(&mut self) -> Ident {
    let name = match self.lift_counter {
      0 => "_liftedReactAsyncImport".into(),
      n => format!("_liftedReactAsyncImport{}", n).into(),
    };

    self.lift_counter += 1;

    Ident::new(
      name,
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(self.unresolved_mark),
    )
  }

  /// Create a lifted variable declarator for an import expression
  fn create_lifted_var(&self, lifted_id: &Ident, import_call: Expr) -> VarDeclarator {
    let mk_ident = |name: &str| {
      Ident::new(
        name.into(),
        DUMMY_SP,
        SyntaxContext::empty().apply_mark(self.unresolved_mark),
      )
    };

    let stmt: Stmt = quote!(
      "const $id = function() { { const fileExports = $interop($import_call); return $promise.resolve(fileExports); } }();" as Stmt,
      id: Ident = lifted_id.clone(),
      interop: Ident = mk_ident("_interopRequireWildcard"),
      import_call: Expr = import_call,
      promise: Ident = mk_ident("Promise")
    );

    match stmt {
      Stmt::Decl(Decl::Var(var_decl)) => var_decl.decls.into_iter().next().unwrap(),
      _ => unreachable!("quote! should generate a var declaration"),
    }
  }

  /// Lift import expressions within a loader function
  fn lift_imports_in_function(&mut self, func: &mut Function) -> Option<Ident> {
    let import_call = func
      .body
      .as_mut()?
      .stmts
      .iter_mut()
      .find_map(|stmt| match stmt {
        Stmt::Return(ret_stmt) => ret_stmt.arg.as_mut().and_then(Self::extract_import_expr),
        _ => None,
      })?;

    let lifted_id = self.generate_lifted_import_id();
    self
      .lifted_imports
      .push(self.create_lifted_var(&lifted_id, *import_call));
    Some(lifted_id)
  }

  /// Extract import expression from various expression types
  fn extract_import_expr(expr: &mut Box<Expr>) -> Option<Box<Expr>> {
    match &mut **expr {
      Expr::Call(call) if matches!(&call.callee, Callee::Import(_)) => {
        // Convert import(arg) to require(arg)
        call.args.first().map(|arg| {
          let arg_expr = arg.expr.clone();
          Box::new(quote!("require($arg)" as Expr, arg: Expr = *arg_expr))
        })
      }
      Expr::Call(call) => {
        // Check if the callee is a member expression (e.g., import().then())
        if let Callee::Expr(expr) = &mut call.callee
          && let Expr::Member(member) = &mut **expr
        {
          Self::extract_import_expr(&mut member.obj)
        } else {
          None
        }
      }
      Expr::Member(member) => Self::extract_import_expr(&mut member.obj),
      _ => None,
    }
  }

  /// Check if the lifted import should be replaced with identifier reference
  fn should_replace_with_lifted(expr: &mut Box<Expr>, lifted_id: &Ident) -> bool {
    match &mut **expr {
      Expr::Call(call) => {
        if matches!(&call.callee, Callee::Import(_)) {
          *expr = Box::new(Expr::Ident(lifted_id.clone()));
          true
        } else if let Callee::Expr(callee_expr) = &mut call.callee
          && let Expr::Member(member) = &mut **callee_expr
        {
          // Handle .then() chains: import().then()
          Self::should_replace_with_lifted(&mut member.obj, lifted_id)
        } else {
          false
        }
      }
      Expr::Member(member) => {
        // For .then() chains, replace the import but keep the .then()
        Self::should_replace_with_lifted(&mut member.obj, lifted_id)
      }
      _ => false,
    }
  }

  /// Lift imports from arrow function expression body and replace with identifier
  fn lift_and_replace_import(&mut self, body_expr: &mut Box<Expr>) {
    if let Some(import_call) = Self::extract_import_expr(body_expr) {
      let lifted_id = self.generate_lifted_import_id();
      self
        .lifted_imports
        .push(self.create_lifted_var(&lifted_id, *import_call));
      Self::should_replace_with_lifted(body_expr, &lifted_id);
    }
  }

  /// Check if an import specifier is for JSResourceForUserVisible
  fn is_jsx_resource_import(named: &ImportNamedSpecifier) -> bool {
    match &named.imported {
      Some(ModuleExportName::Ident(imported)) => &*imported.sym == "JSResourceForUserVisible",
      None => &*named.local.sym == "JSResourceForUserVisible",
      _ => false,
    }
  }

  /// Create an arrow function that returns the lifted import identifier
  fn create_lifted_reference_arrow(_span: Span, lifted_id: Ident) -> Expr {
    quote!("() => $id" as Expr, id = lifted_id)
  }

  /// Handle arrow function with block statement body
  fn handle_arrow_block_body(
    &mut self,
    params: &[Pat],
    span: Span,
    is_async: bool,
    block: &BlockStmt,
  ) -> Option<Box<Expr>> {
    let mut func = Function {
      params: params
        .iter()
        .map(|pat| Param {
          span: DUMMY_SP,
          decorators: vec![],
          pat: pat.clone(),
        })
        .collect(),
      decorators: vec![],
      span,
      ctxt: SyntaxContext::empty(),
      body: Some(block.clone()),
      is_generator: false,
      is_async,
      type_params: None,
      return_type: None,
    };

    self
      .lift_imports_in_function(&mut func)
      .map(|lifted_id| Box::new(Self::create_lifted_reference_arrow(span, lifted_id)))
  }

  /// Process a loader function argument (arrow or function expression)
  fn process_loader(&mut self, loader_expr: &mut Box<Expr>) {
    match &mut **loader_expr {
      Expr::Arrow(arrow) => match &mut *arrow.body {
        BlockStmtOrExpr::Expr(body_expr) => self.lift_and_replace_import(body_expr),
        BlockStmtOrExpr::BlockStmt(block) => {
          let (params, span, is_async) = (arrow.params.clone(), arrow.span, arrow.is_async);
          if let Some(new_expr) = self.handle_arrow_block_body(&params, span, is_async, block) {
            *loader_expr = new_expr;
          }
        }
      },
      Expr::Fn(func_expr) => {
        if let Some(lifted_id) = self.lift_imports_in_function(&mut func_expr.function) {
          *loader_expr = Box::new(Self::create_lifted_reference_arrow(
            func_expr.function.span,
            lifted_id,
          ));
        }
      }
      _ => {}
    }
  }

  pub fn should_transform(file_code: &str) -> bool {
    file_code.contains("@atlassian/react-async")
  }
}

impl VisitMut for ReactAsyncImportLift {
  fn visit_mut_import_decl(&mut self, import: &mut ImportDecl) {
    // Collect JSResourceForUserVisible bindings from @atlassian/react-async imports
    if import.src.value == "@atlassian/react-async" {
      import
        .specifiers
        .iter()
        .filter_map(|spec| match spec {
          ImportSpecifier::Named(named) if Self::is_jsx_resource_import(named) => {
            Some(named.local.to_id())
          }
          _ => None,
        })
        .for_each(|id| {
          self.jsx_resource_bindings.insert(id);
        });
    }
  }

  fn visit_mut_module(&mut self, module: &mut Module) {
    module.visit_mut_children_with(self);

    // Insert lifted imports at the beginning
    if !self.lifted_imports.is_empty() {
      let var_decl = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: self.lifted_imports.clone(),
      }))));
      module.body.insert(0, var_decl);
    }
  }

  fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
    // Check if this is a call to JSResourceForUserVisible with at least 2 arguments
    if let Callee::Expr(callee) = &call.callee
      && let Expr::Ident(ident) = &**callee
      && self.jsx_resource_bindings.contains(&ident.to_id())
      && call.args.len() >= 2
    {
      // Determine if lifting should happen by checking the options
      let should_lift = call.args.get(1).is_some_and(
        |opts_arg| matches!(&*opts_arg.expr, Expr::Object(opts) if self.is_import_lifted(opts)),
      );

      // If lifting is enabled, process the loader argument
      if should_lift && let Some(loader_arg) = call.args.first_mut() {
        self.process_loader(&mut loader_arg.expr);
      }
    }
    call.visit_mut_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_lifts_import_with_flag_enabled() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const MyEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./src/ui/index.tsx'),
          { moduleId: "abc123", entryFsSsrLiftImportToModule: true },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, false, "report".into())
      });

    let expected = indoc! {r#"
      const _liftedReactAsyncImport = function() {
          {
              const fileExports = _interopRequireWildcard(require('./src/ui/index.tsx'));
              return Promise.resolve(fileExports);
          }
      }();
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const MyEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>_liftedReactAsyncImport, {
              moduleId: "abc123",
              entryFsSsrLiftImportToModule: true
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }

  #[test]
  fn test_does_not_lift_without_flag() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const MyEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./src/ui/index.tsx'),
          { moduleId: "abc123" },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, false, "report".into())
      });

    let expected = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const MyEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>import('./src/ui/index.tsx'), {
              moduleId: "abc123"
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }

  #[test]
  fn test_lifts_with_then_chain() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const MyEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./src/ui/index.tsx').then(m => m.MyUIContent),
          { moduleId: "abc123", entryFsSsrLiftImportToModule: true },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, false, "report".into())
      });

    let expected = indoc! {r#"
      const _liftedReactAsyncImport = function() {
          {
              const fileExports = _interopRequireWildcard(require('./src/ui/index.tsx'));
              return Promise.resolve(fileExports);
          }
      }();
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const MyEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>_liftedReactAsyncImport.then((m)=>m.MyUIContent), {
              moduleId: "abc123",
              entryFsSsrLiftImportToModule: true
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }

  #[test]
  fn test_lifts_with_default_enabled() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const MyEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./src/ui/index.tsx'),
          { moduleId: "abc123" },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, true, "report".into())
      });

    let expected = indoc! {r#"
      const _liftedReactAsyncImport = function() {
          {
              const fileExports = _interopRequireWildcard(require('./src/ui/index.tsx'));
              return Promise.resolve(fileExports);
          }
      }();
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const MyEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>_liftedReactAsyncImport, {
              moduleId: "abc123"
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }

  #[test]
  fn test_replaces_arrow_function_without_then() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const MyEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./src/ui/index.tsx'),
          { moduleId: "abc123", entryFsSsrLiftImportToModule: true },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, false, "report".into())
      });

    let expected = indoc! {r#"
      const _liftedReactAsyncImport = function() {
          {
              const fileExports = _interopRequireWildcard(require('./src/ui/index.tsx'));
              return Promise.resolve(fileExports);
          }
      }();
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const MyEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>_liftedReactAsyncImport, {
              moduleId: "abc123",
              entryFsSsrLiftImportToModule: true
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }

  #[test]
  fn test_real_world_with_then_and_destructure() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const nav4DashboardsContentEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./DashboardsContentViewQuery').then(({ DashboardsContentViewQuery }) => DashboardsContentViewQuery),
          { includeInShellOnlySsrBundle: true, entryFsSsrLiftImportToModule: true },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, false, "report".into())
      });

    let expected = indoc! {r#"
      const _liftedReactAsyncImport = function() {
          {
              const fileExports = _interopRequireWildcard(require('./DashboardsContentViewQuery'));
              return Promise.resolve(fileExports);
          }
      }();
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const nav4DashboardsContentEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>_liftedReactAsyncImport.then(({ DashboardsContentViewQuery })=>DashboardsContentViewQuery), {
              includeInShellOnlySsrBundle: true,
              entryFsSsrLiftImportToModule: true
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }

  #[test]
  fn test_explicit_false_overrides_default() {
    let file_code = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';

      export const MyEntryPoint = createEntryPoint({
        root: JSResourceForUserVisible(
          () => import('./src/ui/index.tsx'),
          { moduleId: "abc123", entryFsSsrLiftImportToModule: false },
        ),
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        ReactAsyncImportLift::new(run_test_context.unresolved_mark, true, "report".into())
      });

    let expected = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      import { createEntryPoint } from '@atlassian/react-entrypoint';
      export const MyEntryPoint = createEntryPoint({
          root: JSResourceForUserVisible(()=>import('./src/ui/index.tsx'), {
              moduleId: "abc123",
              entryFsSsrLiftImportToModule: false
          })
      });
    "#};

    assert_eq!(output_code.trim(), expected.trim());
  }
}
