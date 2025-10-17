use swc_core::ecma::ast::*;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

/// Transformer that removes unreachable statements after return statements.
///
/// In functions with multiple return statements, only the first return is reachable.
/// This transform removes all statements that appear after the first return statement
/// in a block, as they can never be executed.
///
/// # Example
///
/// Input:
/// ```js
/// function foo() {
///   console.log('before');
///   return 1;
///   console.log('unreachable');
///   return 2;
/// }
/// ```
///
/// Output:
/// ```js
/// function foo() {
///   console.log('before');
///   return 1;
/// }
/// ```
///
/// # Nested Blocks
///
/// The transformer also handles nested block statements:
///
/// ```js
/// function bar() {
///   if (condition) {
///     return 1;
///     console.log('dead');
///   }
///   return 2;
///   console.log('also dead');
/// }
/// ```
///
/// Output:
/// ```js
/// function bar() {
///   if (condition) {
///     return 1;
///   }
///   return 2;
/// }
/// ```
pub struct DeadReturnsRemover;

impl DeadReturnsRemover {
  pub fn new() -> Self {
    Self
  }

  fn remove_dead_returns(stmts: &mut Vec<Stmt>) {
    if let Some(return_idx) = stmts
      .iter()
      .position(|stmt| matches!(stmt, Stmt::Return(_)))
    {
      stmts.truncate(return_idx + 1);
    }
  }
}

impl VisitMut for DeadReturnsRemover {
  fn visit_mut_function(&mut self, node: &mut Function) {
    node.visit_mut_children_with(self);
    if let Some(body) = &mut node.body {
      Self::remove_dead_returns(&mut body.stmts);
    }
  }

  fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
    node.visit_mut_children_with(self);
    if let BlockStmtOrExpr::BlockStmt(block) = &mut *node.body {
      Self::remove_dead_returns(&mut block.stmts);
    }
  }

  fn visit_mut_block_stmt(&mut self, node: &mut BlockStmt) {
    node.visit_mut_children_with(self);
    Self::remove_dead_returns(&mut node.stmts);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_removes_statements_after_return() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function foo() {
          console.log('before');
          return 1;
          console.log('unreachable');
          return 2;
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function foo() {
            console.log('before');
            return 1;
        }
      "#}
    );
  }

  #[test]
  fn test_removes_dead_returns_in_nested_blocks() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function bar() {
          if (condition) {
            doSomething();
            return 1;
            console.log('dead');
            return 999;
          }
          return 2;
          console.log('also dead');
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function bar() {
            if (condition) {
                doSomething();
                return 1;
            }
            return 2;
        }
      "#}
    );
  }

  #[test]
  fn test_works_with_arrow_functions() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const fn = () => {
          console.log('start');
          return 42;
          console.log('unreachable');
        };
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const fn = ()=>{
            console.log('start');
            return 42;
        };
      "#}
    );
  }

  #[test]
  fn test_preserves_statements_before_return() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function baz() {
          const x = 1;
          const y = 2;
          console.log(x + y);
          return x + y;
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function baz() {
            const x = 1;
            const y = 2;
            console.log(x + y);
            return x + y;
        }
      "#}
    );
  }

  #[test]
  fn test_handles_function_with_no_return() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function noReturn() {
          console.log('line 1');
          console.log('line 2');
          console.log('line 3');
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function noReturn() {
            console.log('line 1');
            console.log('line 2');
            console.log('line 3');
        }
      "#}
    );
  }

  #[test]
  fn test_handles_early_return_in_conditional() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function earlyReturn(x) {
          if (x < 0) {
            console.log('negative');
            return -1;
            console.log('after return in if');
          }
          console.log('positive or zero');
          return x;
          console.log('after final return');
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function earlyReturn(x) {
            if (x < 0) {
                console.log('negative');
                return -1;
            }
            console.log('positive or zero');
            return x;
        }
      "#}
    );
  }

  #[test]
  fn test_handles_nested_functions() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function outer() {
          function inner() {
            return 1;
            console.log('dead in inner');
          }
          return inner();
          console.log('dead in outer');
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function outer() {
            function inner() {
                return 1;
            }
            return inner();
        }
      "#}
    );
  }

  #[test]
  fn test_handles_multiple_nested_blocks() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function complex() {
          if (a) {
            {
              return 1;
              console.log('dead 1');
            }
          }
          return 2;
          console.log('dead 3');
        }
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function complex() {
            if (a) {
                {
                    return 1;
                }
            }
            return 2;
        }
      "#}
    );
  }

  #[test]
  fn test_empty_function() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function empty() {}
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function empty() {}
      "#}
    );
  }

  #[test]
  fn test_arrow_function_with_expression_body() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const fn = () => 42;
      "#},
      |_: RunTestContext| DeadReturnsRemover::new(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const fn = ()=>42;
      "#}
    );
  }
}
