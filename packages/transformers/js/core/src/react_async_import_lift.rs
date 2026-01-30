use std::collections::HashSet;

use swc_core::common::{DUMMY_SP, Mark, SourceMap, Span, SyntaxContext, sync::Lrc};
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_core::quote;

use crate::utils::{CodeHighlight, Diagnostic, DiagnosticSeverity, SourceLocation};

/// Transformer that lifts dynamic imports out of JSResourceForUserVisible calls
/// from @atlassian/react-async for SSR optimization.
///
/// This transform enables SSR optimization by hoisting `import()` calls inside
/// JSResourceForUserVisible loader functions to the module scope.
/// Lifting is controlled by the `entryFsSsrLiftImportToModule` option.
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
/// Output (after this transformer):
/// ```js
/// const _liftedReactAsyncImport = import('./src/ui/index.tsx');
/// import { JSResourceForUserVisible } from '@atlassian/react-async';
/// export const MyEntryPoint = createEntryPoint({
///   root: JSResourceForUserVisible(() => _liftedReactAsyncImport, {
///     moduleId: "abc123",
///     entryFsSsrLiftImportToModule: true
///   }),
/// });
/// ```
pub struct ReactAsyncImportLift<'a> {
  /// Mark used for generated/synthetic nodes
  pub global_mark: Mark,
  /// Whether import lifting is applied regardless of import options
  pub import_lifting_by_default: bool,
  pub reporting_level: String,
  /// Collected lifted imports to be inserted at module scope
  pub lifted_imports: Vec<VarDeclarator>,
  /// Counter for generating unique lifted import identifiers
  pub lift_counter: usize,
  /// Set to track bindings of JSResourceForUserVisible imports
  pub jsx_resource_bindings: HashSet<Id>,
  /// Source map for location information
  pub source_map: Lrc<SourceMap>,
  /// Diagnostics collected during transformation
  pub diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> ReactAsyncImportLift<'a> {
  pub fn new(
    global_mark: Mark,
    import_lifting_by_default: bool,
    reporting_level: String,
    source_map: Lrc<SourceMap>,
    diagnostics: &'a mut Vec<Diagnostic>,
  ) -> Self {
    Self {
      global_mark,
      import_lifting_by_default,
      reporting_level,
      lifted_imports: Vec::new(),
      lift_counter: 0,
      jsx_resource_bindings: HashSet::new(),
      source_map,
      diagnostics,
    }
  }

  /// Check if import lifting is enabled for a given options object
  fn is_import_lifted(&mut self, opts: &ObjectLit, call_span: Span) -> bool {
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

    // Report missing flag if configured with diagnostic
    if matches!(self.reporting_level.as_str(), "report" | "error") {
      let loc = SourceLocation::from(&self.source_map, call_span);

      let severity = match self.reporting_level.as_str() {
        "report" => DiagnosticSeverity::Warning,
        "error" => DiagnosticSeverity::Error,
        _ => unreachable!(),
      };

      self.diagnostics.push(Diagnostic {
        message: "No entryFsSsrLiftImportToModule setting found in JSResourceForUserVisible call"
          .into(),
        code_highlights: Some(vec![CodeHighlight { message: None, loc }]),
        hints: Some(vec![
          "Add { entryFsSsrLiftImportToModule: true } to the options object".into(),
          "Or set { entryFsSsrLiftImportToModule: false } to explicitly disable lifting".into(),
        ]),
        show_environment: false,
        severity,
        documentation_url: None,
      });
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

    // Use global_mark for generated/synthetic identifiers
    // Unresolved_mark is for imports and external references, not local declarations
    Ident::new(
      name,
      DUMMY_SP,
      SyntaxContext::empty().apply_mark(self.global_mark),
    )
  }

  /// Lift imports from an expression, replacing import() calls with lifted identifiers
  /// Returns Some(lifted_id) if the expression becomes a simple identifier after lifting
  fn lift_imports_in_expr(&mut self, expr: &mut Box<Expr>) -> Option<Ident> {
    // Find and extract the import() call
    let import_call = Self::find_and_extract_import(expr)?;

    let lifted_id = self.generate_lifted_import_id();

    // Create the lifted variable declarator
    self.lifted_imports.push(VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Ident(BindingIdent {
        id: lifted_id.clone(),
        type_ann: None,
      }),
      init: Some(import_call),
      definite: false,
    });

    // Replace import() with the lifted identifier in the expression tree
    Self::replace_import_with_ident(expr, &lifted_id);

    // Return the lifted_id if the expression is now just an identifier
    matches!(**expr, Expr::Ident(_)).then(|| lifted_id)
  }

  /// Find and extract import() call from an expression
  /// Recursively searches through member expressions and call chains
  fn find_and_extract_import(expr: &Expr) -> Option<Box<Expr>> {
    match expr {
      Expr::Call(call) if matches!(call.callee, Callee::Import(_)) => {
        Some(Box::new(Expr::Call(call.clone())))
      }
      Expr::Call(call) => {
        if let Callee::Expr(callee_expr) = &call.callee
          && let Expr::Member(member) = &**callee_expr
        {
          Self::find_and_extract_import(&member.obj)
        } else {
          None
        }
      }
      Expr::Member(member) => Self::find_and_extract_import(&member.obj),
      _ => None,
    }
  }

  /// Replace import() call with identifier in expression tree
  fn replace_import_with_ident(expr: &mut Box<Expr>, lifted_id: &Ident) {
    match &mut **expr {
      Expr::Call(call) if matches!(call.callee, Callee::Import(_)) => {
        *expr = Box::new(Expr::Ident(lifted_id.clone()));
      }
      Expr::Call(call) => {
        if let Callee::Expr(callee_expr) = &mut call.callee
          && let Expr::Member(member) = &mut **callee_expr
        {
          Self::replace_import_with_ident(&mut member.obj, lifted_id);
        }
      }
      Expr::Member(member) => {
        Self::replace_import_with_ident(&mut member.obj, lifted_id);
      }
      _ => {}
    }
  }

  /// Lift imports from statements (handles conditionals, blocks, returns)
  fn lift_imports_in_stmt(&mut self, stmt: &mut Stmt) {
    match stmt {
      Stmt::Return(ret_stmt) => {
        if let Some(arg) = ret_stmt.arg.as_mut() {
          self.lift_imports_in_expr(arg);
        }
      }
      Stmt::If(if_stmt) => {
        self.lift_imports_in_stmt(&mut if_stmt.cons);
        if let Some(alt) = &mut if_stmt.alt {
          self.lift_imports_in_stmt(alt);
        }
        // Also check test expression
        self.lift_imports_in_expr(&mut if_stmt.test);
      }
      Stmt::Block(block) => {
        for stmt in &mut block.stmts {
          self.lift_imports_in_stmt(stmt);
        }
      }
      Stmt::Expr(expr_stmt) => {
        self.lift_imports_in_expr(&mut expr_stmt.expr);
      }
      _ => {}
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

  /// Process a loader function argument (arrow or function expression)
  /// Lifts all import() calls and simplifies the loader if possible
  fn process_loader(&mut self, loader_expr: &mut Box<Expr>) {
    match &mut **loader_expr {
      Expr::Arrow(arrow) => {
        match &mut *arrow.body {
          BlockStmtOrExpr::Expr(body_expr) => {
            // Lift imports from expression body
            if let Some(lifted_id) = self.lift_imports_in_expr(body_expr) {
              // Body is now just an identifier, replace entire arrow with simpler one
              *loader_expr = Box::new(quote!("() => $id" as Expr, id = lifted_id));
            }
          }
          BlockStmtOrExpr::BlockStmt(block) => {
            // Lift imports from block statements
            for stmt in &mut block.stmts {
              self.lift_imports_in_stmt(stmt);
            }

            // Simplify if block is now just `return lifted_id`
            if let [Stmt::Return(ReturnStmt { arg: Some(arg), .. })] = &block.stmts[..]
              && let Expr::Ident(ident) = &**arg
            {
              *loader_expr = Box::new(quote!("() => $id" as Expr, id = ident.clone()));
            }
          }
        }
      }
      Expr::Fn(FnExpr { function, .. }) => {
        // Lift imports from function body
        if let Some(body) = &mut function.body {
          for stmt in &mut body.stmts {
            self.lift_imports_in_stmt(stmt);
          }

          // Simplify if function is now just `return lifted_id`
          if let [Stmt::Return(ReturnStmt { arg: Some(arg), .. })] = &body.stmts[..]
            && let Expr::Ident(ident) = &**arg
          {
            *loader_expr = Box::new(quote!("() => $id" as Expr, id = ident.clone()));
          }
        }
      }
      _ => {}
    }
  }

  pub fn should_transform(file_code: &str) -> bool {
    file_code.contains("@atlassian/react-async")
  }
}

impl VisitMut for ReactAsyncImportLift<'_> {
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
        |opts_arg| matches!(&*opts_arg.expr, Expr::Object(opts) if self.is_import_lifted(opts, call.span)),
      );

      // If lifting is enabled, process the loader argument and DON'T visit children
      // to avoid re-processing the lifted imports we just created
      if should_lift && let Some(loader_arg) = call.args.first_mut() {
        self.process_loader(&mut loader_arg.expr);
        return;
      }
    }
    call.visit_mut_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, run_test_visit};
  use indoc::indoc;

  /// Helper to run transformer with default settings
  fn run_transform(code: &str, lift_by_default: bool) -> String {
    let mut diagnostics = Vec::new();
    run_test_visit(code, |ctx: RunTestContext| {
      ReactAsyncImportLift::new(
        ctx.global_mark,
        lift_by_default,
        "report".into(),
        ctx.source_map,
        &mut diagnostics,
      )
    })
    .output_code
  }

  /// Helper to run transformer with custom reporting level and return diagnostics
  fn run_transform_with_reporting(
    code: &str,
    lift_by_default: bool,
    reporting_level: &str,
  ) -> (String, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let output = run_test_visit(code, |ctx: RunTestContext| {
      ReactAsyncImportLift::new(
        ctx.global_mark,
        lift_by_default,
        reporting_level.into(),
        ctx.source_map,
        &mut diagnostics,
      )
    })
    .output_code;
    (output, diagnostics)
  }

  #[test]
  fn test_basic_import_lifting() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { moduleId: "abc123", entryFsSsrLiftImportToModule: true }
      );
    "#};

    let output = run_transform(input, false);

    assert!(output.contains("const _liftedReactAsyncImport = import('./ui/index.tsx')"));
    assert!(output.contains("()=>_liftedReactAsyncImport"));
  }

  #[test]
  fn test_no_lifting_without_flag() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { moduleId: "abc123" }
      );
    "#};

    let output = run_transform(input, false);

    assert!(!output.contains("_liftedReactAsyncImport"));
    assert!(output.contains("()=>import('./ui/index.tsx')"));
  }

  #[test]
  fn test_lifting_preserves_then_chain() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx').then(m => m.MyUIContent),
        { entryFsSsrLiftImportToModule: true }
      );
    "#};

    let output = run_transform(input, false);

    assert!(output.contains("const _liftedReactAsyncImport = import('./ui/index.tsx')"));
    assert!(output.contains("()=>_liftedReactAsyncImport.then"));
    assert!(output.contains("m.MyUIContent"));
  }

  #[test]
  fn test_default_lifting_enabled() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { moduleId: "abc123" }
      );
    "#};

    let output = run_transform(input, true);

    assert!(output.contains("const _liftedReactAsyncImport = import('./ui/index.tsx')"));
    assert!(output.contains("()=>_liftedReactAsyncImport"));
  }

  #[test]
  fn test_explicit_false_overrides_default() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { entryFsSsrLiftImportToModule: false }
      );
    "#};

    let output = run_transform(input, true);

    assert!(!output.contains("_liftedReactAsyncImport"));
    assert!(output.contains("()=>import('./ui/index.tsx')"));
  }

  #[test]
  fn test_multiple_imports_with_unique_names() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const Entry1 = JSResourceForUserVisible(() => import('./file1.tsx'), { entryFsSsrLiftImportToModule: true });
      export const Entry2 = JSResourceForUserVisible(() => import('./file2.tsx'), { entryFsSsrLiftImportToModule: true });
      export const Entry3 = JSResourceForUserVisible(() => import('./file3.tsx'), { entryFsSsrLiftImportToModule: true });
    "#};

    let output = run_transform(input, false);

    assert!(output.contains("_liftedReactAsyncImport = "));
    assert!(output.contains("_liftedReactAsyncImport1 = "));
    assert!(output.contains("_liftedReactAsyncImport2 = "));
    assert!(output.contains("Entry1 = JSResourceForUserVisible(()=>_liftedReactAsyncImport"));
    assert!(output.contains("Entry2 = JSResourceForUserVisible(()=>_liftedReactAsyncImport1"));
    assert!(output.contains("Entry3 = JSResourceForUserVisible(()=>_liftedReactAsyncImport2"));
  }

  #[test]
  fn test_conditional_imports_all_lifted() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => {
          if (someCondition) {
            return import('./file1.tsx');
          }
          return import('./file2.tsx');
        },
        { entryFsSsrLiftImportToModule: true }
      );
    "#};

    let output = run_transform(input, false);

    assert!(output.contains("_liftedReactAsyncImport = import('./file1.tsx')"));
    assert!(output.contains("_liftedReactAsyncImport1 = import('./file2.tsx')"));
    assert!(output.contains("if (someCondition)"));
    assert!(output.contains("return _liftedReactAsyncImport"));
    assert!(output.contains("return _liftedReactAsyncImport1"));
  }

  #[test]
  fn test_diagnostics_with_different_severity_levels() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { moduleId: "abc123" }
      );
    "#};

    // Test warning severity
    let (_output, diagnostics) = run_transform_with_reporting(input, false, "report");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
    assert!(
      diagnostics[0]
        .message
        .contains("No entryFsSsrLiftImportToModule setting found")
    );
    assert_eq!(
      diagnostics[0].code_highlights.as_ref().unwrap()[0]
        .loc
        .start_line,
      2
    );

    // Test error severity (causes build failure)
    let (_output, diagnostics) = run_transform_with_reporting(input, false, "error");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    assert!(
      diagnostics[0]
        .message
        .contains("No entryFsSsrLiftImportToModule setting found")
    );

    // Verify hints are provided
    assert!(diagnostics[0].hints.is_some());
    let hints = diagnostics[0].hints.as_ref().unwrap();
    assert!(
      hints
        .iter()
        .any(|h| h.contains("entryFsSsrLiftImportToModule"))
    );
  }

  #[test]
  fn test_no_diagnostic_when_flag_present() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { entryFsSsrLiftImportToModule: true }
      );
    "#};

    let (_output, diagnostics) = run_transform_with_reporting(input, false, "report");

    // No diagnostic should be created when flag is explicitly set
    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn test_no_diagnostic_when_reporting_disabled() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const MyEntryPoint = JSResourceForUserVisible(
        () => import('./ui/index.tsx'),
        { moduleId: "abc123" }
      );
    "#};

    let (_output, diagnostics) = run_transform_with_reporting(input, false, "off");

    // No diagnostic should be created when reporting level is "off"
    assert_eq!(diagnostics.len(), 0);
  }

  #[test]
  fn test_multiple_calls_create_multiple_diagnostics() {
    let input = indoc! {r#"
      import { JSResourceForUserVisible } from '@atlassian/react-async';
      export const Entry1 = JSResourceForUserVisible(() => import('./file1.tsx'), { moduleId: "1" });
      export const Entry2 = JSResourceForUserVisible(() => import('./file2.tsx'), { moduleId: "2" });
    "#};

    let (_output, diagnostics) = run_transform_with_reporting(input, false, "report");

    // Should create one diagnostic for each call
    assert_eq!(diagnostics.len(), 2);
    assert!(
      diagnostics
        .iter()
        .all(|d| d.severity == DiagnosticSeverity::Warning)
    );

    // Verify different line numbers for each diagnostic
    let line1 = diagnostics[0].code_highlights.as_ref().unwrap()[0]
      .loc
      .start_line;
    let line2 = diagnostics[1].code_highlights.as_ref().unwrap()[0]
      .loc
      .start_line;
    assert_eq!(line1, 2); // First JSResourceForUserVisible call
    assert_eq!(line2, 3); // Second JSResourceForUserVisible call
  }
}
