use swc_core::common::DUMMY_SP;
use swc_core::common::Mark;
use swc_core::common::SyntaxContext;
use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

/// Transforms dynamic imports for lazy loading libraries.
/// Equivalent to the @atlassian/react-loosely-lazy handling from the AFM Babel plugin.
pub struct LazyLoadingTransformer {
  pub unresolved_mark: Mark,
}

impl Default for LazyLoadingTransformer {
  fn default() -> Self {
    Self {
      unresolved_mark: Mark::root(),
    }
  }
}

impl LazyLoadingTransformer {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self { unresolved_mark }
  }

  pub fn should_transform(file_code: &str) -> bool {
    file_code.contains("@atlassian/react-loosely-lazy")
  }
}

impl VisitMut for LazyLoadingTransformer {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    // Transform dynamic import() calls to require() calls for lazy loading
    if let Expr::Call(call) = node
      && let Callee::Import(_) = &call.callee
    {
      // Replace import() with require()
      *node = Expr::Call(CallExpr {
        span: call.span,
        callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
          "require".into(),
          DUMMY_SP,
          SyntaxContext::empty().apply_mark(self.unresolved_mark),
        )))),
        args: call.args.clone(),
        type_args: None,
        ctxt: call.ctxt,
      });
      return;
    }

    node.visit_mut_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_should_transform_with_package() {
    assert!(LazyLoadingTransformer::should_transform(
      "import { lazy } from '@atlassian/react-loosely-lazy';"
    ));
    assert!(LazyLoadingTransformer::should_transform(
      "const x = require('@atlassian/react-loosely-lazy');"
    ));
    assert!(LazyLoadingTransformer::should_transform(
      "// some code\nimport { lazy } from '@atlassian/react-loosely-lazy';\nmore code"
    ));
  }

  #[test]
  fn test_should_transform_without_package() {
    assert!(!LazyLoadingTransformer::should_transform(
      "import { lazy } from 'react';"
    ));
    assert!(!LazyLoadingTransformer::should_transform(
      "const Component = lazy(() => import('./Component'));"
    ));
    assert!(!LazyLoadingTransformer::should_transform(""));
    assert!(!LazyLoadingTransformer::should_transform(
      "import something from '@atlassian/other-package';"
    ));
  }

  #[test]
  fn test_lazy_loading_transform_when_package_present() {
    let file_code = indoc! {r#"
      import { lazy } from '@atlassian/react-loosely-lazy';
      const Component = lazy(() => import('./Component'));
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        LazyLoadingTransformer::new(run_test_context.unresolved_mark)
      });

    assert_eq!(
      output_code,
      indoc! {r#"
        import { lazy } from '@atlassian/react-loosely-lazy';
        const Component = lazy(()=>require('./Component'));
      "#}
    );
  }

  #[test]
  fn test_multiple_dynamic_imports() {
    let file_code = indoc! {r#"
      import { lazy } from '@atlassian/react-loosely-lazy';
      const Component1 = lazy(() => import('./Component1'));
      const Component2 = lazy(() => import('./Component2'));
      const regularImport = import('./regular');
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        LazyLoadingTransformer::new(run_test_context.unresolved_mark)
      });

    assert_eq!(
      output_code,
      indoc! {r#"
        import { lazy } from '@atlassian/react-loosely-lazy';
        const Component1 = lazy(()=>require('./Component1'));
        const Component2 = lazy(()=>require('./Component2'));
        const regularImport = require('./regular');
      "#}
    );
  }

  #[test]
  fn test_nested_dynamic_imports() {
    let file_code = indoc! {r#"
      import { lazy } from '@atlassian/react-loosely-lazy';
      const Component = lazy(async () => {
        const module = await import('./Component');
        return module.default;
      });
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(file_code, |run_test_context: RunTestContext| {
        LazyLoadingTransformer::new(run_test_context.unresolved_mark)
      });

    assert_eq!(
      output_code,
      indoc! {r#"
        import { lazy } from '@atlassian/react-loosely-lazy';
        const Component = lazy(async ()=>{
            const module = await require('./Component');
            return module.default;
        });
      "#}
    );
  }
}
