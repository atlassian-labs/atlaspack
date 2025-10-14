use std::collections::HashSet;

use swc_core::common::Mark;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::utils::*;

/// Transformer that removes React hooks and related SSR-incompatible function calls.
///
/// This transform enables SSR compatibility by removing or replacing calls to React
/// hooks that cannot safely execute on the server (eg. `useEffect`, `useLayoutEffect`,
/// `useIsomorphicLayoutEffect`, `di`). These hooks are either removed entirely (when used
/// as statements) or replaced with `undefined` (when used in expressions).
///
/// # Example
///
/// Input:
/// ```js
/// useEffect(() => {
///   console.log('effect');
/// }, []);
///
/// const cleanup = useLayoutEffect(() => {
///   return () => {};
/// });
/// ```
///
/// Output:
/// ```js
/// ;
///
/// const cleanup = undefined;
/// ```
///
/// # Scope Awareness
///
/// Only removes unresolved (global/imported) references. Local variables with the same
/// names are not affected:
///
/// ```js
/// function Component() {
///   const useEffect = () => console.log('local');
///   useEffect(); // Not removed
/// }
/// useEffect(() => {}); // Removed
/// ```
pub struct ReactHooksRemover {
  /// Set of hook identifiers to remove
  pub hooks_to_remove: HashSet<JsWord>,
  /// Mark used to identify unresolved (global/imported) references
  pub unresolved_mark: Mark,
}

impl Default for ReactHooksRemover {
  fn default() -> Self {
    Self {
      hooks_to_remove: HashSet::from([
        "useLayoutEffect".into(),
        "useEffect".into(),
        "useIsomorphicLayoutEffect".into(),
        "di".into(),
      ]),
      unresolved_mark: Mark::root(),
    }
  }
}

impl ReactHooksRemover {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      ..Default::default()
    }
  }

  #[allow(dead_code)]
  pub fn with_hooks(unresolved_mark: Mark, hooks: HashSet<JsWord>) -> Self {
    Self {
      hooks_to_remove: hooks,
      unresolved_mark,
    }
  }

  fn should_remove_call(&self, callee: &Callee) -> bool {
    if let Callee::Expr(expr) = callee
      && let Expr::Ident(ident) = &**expr
    {
      // Only remove if it's an unresolved reference (imported/global) and in our removal list
      return is_unresolved(ident, self.unresolved_mark)
        && self.hooks_to_remove.contains(&ident.sym);
    }

    false
  }
}

impl VisitMut for ReactHooksRemover {
  fn visit_mut_stmt(&mut self, node: &mut Stmt) {
    // Handle expression statements that are just hook calls
    if let Stmt::Expr(expr_stmt) = node
      && let Expr::Call(call) = &*expr_stmt.expr
      && self.should_remove_call(&call.callee)
    {
      // Replace with empty statement
      *node = Stmt::Empty(EmptyStmt {
        span: expr_stmt.span,
      });
      return;
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_expr(&mut self, node: &mut Expr) {
    // Handle hook calls in expressions (e.g. variable assignments)
    if let Expr::Call(call) = node
      && self.should_remove_call(&call.callee)
    {
      // Replace with undefined identifier
      *node = Expr::Ident(get_undefined_ident(self.unresolved_mark));
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
  fn test_react_hooks_removal() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        useEffect(() => {
          console.log('effect');
        }, []);

        useLayoutEffect(() => {
          console.log('layout effect');
        });

        const cleanup = useIsomorphicLayoutEffect(() => {
          return () => {};
        });

        di();

        const other = someOtherFunction();
      "#},
      |run_test_context: RunTestContext| ReactHooksRemover::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        ;
        ;
        const cleanup = undefined;
        ;
        const other = someOtherFunction();
      "#}
    );
  }

  #[test]
  fn test_scoped_functions_not_removed() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function Component() {
          const useEffect = () => console.log('local');
          useEffect();
          return null;
        }

        // Global useEffect should be removed
        useEffect(() => {});
      "#},
      |run_test_context: RunTestContext| ReactHooksRemover::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function Component() {
            const useEffect = ()=>console.log('local');
            useEffect();
            return null;
        }
        ;
      "#}
    );
  }

  #[test]
  fn test_custom_hooks() {
    let mut custom_hooks = HashSet::new();
    custom_hooks.insert("useCustomHook".into());
    custom_hooks.insert("useAnotherHook".into());

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        useCustomHook();
        const result = useAnotherHook();
        useEffect(() => {}); // Should not be removed with custom config
      "#},
      |run_test_context: RunTestContext| {
        ReactHooksRemover::with_hooks(run_test_context.unresolved_mark, custom_hooks)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        ;
        const result = undefined;
        useEffect(()=>{});
      "#}
    );
  }

  #[test]
  fn test_hooks_in_complex_expressions() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const value = someCondition ? useEffect() : null;
        const obj = {
          effect: useLayoutEffect(() => {})
        };

        if (condition) {
          useIsomorphicLayoutEffect();
        }
      "#},
      |run_test_context: RunTestContext| ReactHooksRemover::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const value = someCondition ? undefined : null;
        const obj = {
            effect: undefined
        };
        if (condition) {
            ;
        }
      "#}
    );
  }
}
