use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::Atom;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

/// Pre-evaluates code expressions with static values.
///
/// # Transformations
///
/// - **Binary equality expressions**: `1 === 1` → `true`, `"a" !== "b"` → `true`
/// - **Unary NOT expressions**: `!true` → `false`, `!"string"` → `false`
/// - **Logical expressions**: `true && false` → `false`, `"a" || "b"` → `"a"`
/// - **If statements**: `if (true) { ... }` → `{ ... }`, `if (false) { ... }` → (removed or alternate)
/// - **Conditional expressions**: `true ? a : b` → `a`, `false ? a : b` → `b`
///
/// # Example
///
/// ## Input
/// ```js
/// const x = 1 === 1;
/// const y = !false;
/// const z = "hello" && "world";
/// if (true) {
///   console.log("yes");
/// }
/// ```
///
/// ## Output
/// ```js
/// const x = true;
/// const y = true;
/// const z = "world";
/// {
///   console.log("yes");
/// }
/// ```
pub struct StaticPreEvaluator;

impl StaticPreEvaluator {
  /// Helper to get a literal value for comparison purposes.
  fn get_literal_value(expr: &Expr) -> Option<LiteralValue> {
    match expr {
      Expr::Lit(Lit::Str(s)) => Some(LiteralValue::Str(s.value.clone())),
      Expr::Lit(Lit::Bool(b)) => Some(LiteralValue::Bool(b.value)),
      Expr::Lit(Lit::Num(n)) => Some(LiteralValue::Num(n.value)),
      Expr::Lit(Lit::Null(_)) => Some(LiteralValue::Null),
      _ => None,
    }
  }
}

/// Internal representation of literal values for comparison.
#[derive(Debug, Clone, PartialEq)]
enum LiteralValue {
  Str(Atom),
  Bool(bool),
  Num(f64),
  Null,
}

impl LiteralValue {
  /// Check if this value has the same variant as another (same type).
  fn same_type(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }

  /// Convert to boolean for logical operations.
  fn to_bool(&self) -> bool {
    match self {
      LiteralValue::Bool(b) => *b,
      LiteralValue::Str(s) => !s.is_empty(),
      LiteralValue::Num(n) => *n != 0.0 && !n.is_nan(),
      LiteralValue::Null => false,
    }
  }

  /// Convert back to an Expr.
  fn to_expr(&self, span: swc_core::common::Span) -> Expr {
    match self {
      LiteralValue::Bool(b) => Expr::Lit(Lit::Bool(Bool { span, value: *b })),
      LiteralValue::Str(s) => Expr::Lit(Lit::Str(Str {
        span,
        value: s.clone(),
        raw: None,
      })),
      LiteralValue::Num(n) => Expr::Lit(Lit::Num(Number {
        span,
        value: *n,
        raw: None,
      })),
      LiteralValue::Null => Expr::Lit(Lit::Null(Null { span })),
    }
  }
}

impl StaticPreEvaluator {
  fn eval_binary_expr(&mut self, bin: &BinExpr) -> Option<Expr> {
    let left_val = Self::get_literal_value(&bin.left)?;
    let right_val = Self::get_literal_value(&bin.right)?;

    // Only compare literals of the same type (as in the Babel version)
    if !left_val.same_type(&right_val) {
      return None;
    }

    let result = match bin.op {
      BinaryOp::EqEq | BinaryOp::EqEqEq => left_val == right_val,
      BinaryOp::NotEq | BinaryOp::NotEqEq => left_val != right_val,
      _ => return None,
    };

    Some(Expr::Lit(Lit::Bool(Bool {
      span: bin.span,
      value: result,
    })))
  }

  fn eval_unary_expr(&mut self, unary: &UnaryExpr) -> Option<Expr> {
    if unary.op != UnaryOp::Bang {
      return None;
    }

    let arg_val = Self::get_literal_value(&unary.arg)?;
    let result = !arg_val.to_bool();

    Some(Expr::Lit(Lit::Bool(Bool {
      span: unary.span,
      value: result,
    })))
  }

  fn eval_conditional_expr(&mut self, cond: &CondExpr) -> Option<Expr> {
    match &*cond.test {
      Expr::Lit(Lit::Bool(test_bool)) => Some(if test_bool.value {
        (*cond.cons).clone()
      } else {
        (*cond.alt).clone()
      }),
      _ => None,
    }
  }

  fn eval_if_stmt(&mut self, if_stmt: &IfStmt) -> Option<Stmt> {
    let test_val = Self::get_literal_value(&if_stmt.test)?;

    Some(if test_val.to_bool() {
      (*if_stmt.cons).clone()
    } else {
      if_stmt
        .alt
        .as_ref()
        .map(|alt| (**alt).clone())
        .unwrap_or_else(|| {
          Stmt::Block(BlockStmt {
            span: if_stmt.span,
            stmts: vec![],
            ..Default::default()
          })
        })
    })
  }

  /// Evaluates logical expressions (&&, ||, ??) with literal operands.
  fn eval_logical_expr(&mut self, logical: &BinExpr) -> Option<Expr> {
    let left_val = Self::get_literal_value(&logical.left);
    let right_val = Self::get_literal_value(&logical.right);

    // Special case: && with one falsy literal (short-circuit evaluation)
    // Match Babel semantics: exclude null (it doesn't have a .value property in Babel)
    if logical.op == BinaryOp::LogicalAnd {
      if let Some(ref left) = left_val
        && !left.to_bool()
      {
        return Some((*logical.left).clone());
      }
      if let Some(ref right) = right_val
        && !matches!(right, LiteralValue::Null)
        && !right.to_bool()
      {
        return Some((*logical.right).clone());
      }
    }

    // Both operands must be literals for full evaluation
    let left_val = left_val?;
    let right_val = right_val?;

    let result_val = match logical.op {
      BinaryOp::LogicalAnd => {
        if left_val.to_bool() {
          right_val
        } else {
          left_val
        }
      }
      BinaryOp::LogicalOr => {
        if left_val.to_bool() {
          left_val
        } else {
          right_val
        }
      }
      BinaryOp::NullishCoalescing => {
        if matches!(left_val, LiteralValue::Null) {
          right_val
        } else {
          left_val
        }
      }
      _ => return None,
    };

    Some(result_val.to_expr(logical.span))
  }
}

impl VisitMut for StaticPreEvaluator {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    // Visit children first (bottom-up approach, matching Babel's exit strategy)
    node.visit_mut_children_with(self);

    let replacement = match node {
      Expr::Bin(bin_expr) => match bin_expr.op {
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing => {
          self.eval_logical_expr(bin_expr)
        }
        _ => self.eval_binary_expr(bin_expr),
      },
      Expr::Unary(unary_expr) => self.eval_unary_expr(unary_expr),
      Expr::Cond(cond_expr) => self.eval_conditional_expr(cond_expr),
      _ => None,
    };

    if let Some(replacement) = replacement {
      *node = replacement;
    }
  }

  fn visit_mut_stmt(&mut self, node: &mut Stmt) {
    // Visit children first
    node.visit_mut_children_with(self);

    if let Stmt::If(if_stmt) = node
      && let Some(replacement) = self.eval_if_stmt(if_stmt)
    {
      *node = replacement;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_binary_strict_equality() {
    let code = indoc! {r#"
      const a = 1 === 1;
      const b = "hello" === "hello";
      const c = true === true;
      const d = null === null;
      const e = 1 === 2;
      const f = "a" === "b";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = true;
        const b = true;
        const c = true;
        const d = true;
        const e = false;
        const f = false;
      "#}
    );
  }

  #[test]
  fn test_binary_strict_inequality() {
    let code = indoc! {r#"
      const a = 1 !== 1;
      const b = "hello" !== "world";
      const c = true !== false;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = true;
        const c = true;
      "#}
    );
  }

  #[test]
  fn test_binary_loose_equality() {
    let code = indoc! {r#"
      const a = 1 == 1;
      const b = 1 == 2;
      const c = "a" != "b";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = true;
        const b = false;
        const c = true;
      "#}
    );
  }

  #[test]
  fn test_unary_not_boolean() {
    let code = indoc! {r#"
      const a = !true;
      const b = !false;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = true;
      "#}
    );
  }

  #[test]
  fn test_unary_not_string() {
    let code = indoc! {r#"
      const a = !"hello";
      const b = !"";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = true;
      "#}
    );
  }

  #[test]
  fn test_unary_not_number() {
    let code = indoc! {r#"
      const a = !1;
      const b = !0;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = true;
      "#}
    );
  }

  #[test]
  fn test_logical_and_with_literals() {
    let code = indoc! {r#"
      const a = true && true;
      const b = true && false;
      const c = false && true;
      const d = "hello" && "world";
      const e = "" && "world";
      const f = 1 && 2;
      const g = 0 && 1;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = true;
        const b = false;
        const c = false;
        const d = "world";
        const e = "";
        const f = 2;
        const g = 0;
      "#}
    );
  }

  #[test]
  fn test_logical_and_right_side_null() {
    let code = indoc! {r#"
      const foo = val == "foo" && null;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const foo = val == "foo" && null;
      "#}
    );
  }

  #[test]
  fn test_logical_and_right_side_short_circuit() {
    // Test that right-side short-circuit works with non-null falsy values
    let code = indoc! {r#"
      const a = unknownVar && false;
      const b = unknownVar && 0;
      const c = unknownVar && "";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = 0;
        const c = "";
      "#}
    );
  }

  #[test]
  fn test_logical_or_with_literals() {
    let code = indoc! {r#"
      const a = true || false;
      const b = false || true;
      const c = "hello" || "world";
      const d = "" || "world";
      const e = 0 || 1;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = true;
        const b = true;
        const c = "hello";
        const d = "world";
        const e = 1;
      "#}
    );
  }

  #[test]
  fn test_nullish_coalescing() {
    let code = indoc! {r#"
      const a = null ?? "fallback";
      const b = "value" ?? "fallback";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = "fallback";
        const b = "value";
      "#}
    );
  }

  #[test]
  fn test_logical_and_partial_evaluation() {
    let code = indoc! {r#"
      const a = false && unknownVar;
      const b = 0 && unknownVar;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = 0;
      "#}
    );
  }

  #[test]
  fn test_conditional_expression_true() {
    let code = indoc! {r#"
      const a = true ? "yes" : "no";
      const b = false ? "yes" : "no";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = "yes";
        const b = "no";
      "#}
    );
  }

  #[test]
  fn test_if_statement_true_condition() {
    let code = indoc! {r#"
      if (true) {
        console.log("yes");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        {
            console.log("yes");
        }
      "#}
      .trim_end()
    );
  }

  #[test]
  fn test_if_statement_false_condition() {
    let code = indoc! {r#"
      if (false) {
        console.log("no");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(output_code, "{}");
  }

  #[test]
  fn test_if_statement_with_else() {
    let code = indoc! {r#"
      if (true) {
        console.log("yes");
      } else {
        console.log("no");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        {
            console.log("yes");
        }
      "#}
      .trim_end()
    );
  }

  #[test]
  fn test_if_statement_false_with_else() {
    let code = indoc! {r#"
      if (false) {
        console.log("yes");
      } else {
        console.log("no");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        {
            console.log("no");
        }
      "#}
      .trim_end()
    );
  }

  #[test]
  fn test_if_statement_string_truthy() {
    let code = indoc! {r#"
      if ("hello") {
        console.log("truthy");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        {
            console.log("truthy");
        }
      "#}
      .trim_end()
    );
  }

  #[test]
  fn test_if_statement_string_falsy() {
    let code = indoc! {r#"
      if ("") {
        console.log("truthy");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(output_code, "{}");
  }

  #[test]
  fn test_if_statement_number_truthy() {
    let code = indoc! {r#"
      if (1) {
        console.log("truthy");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        {
            console.log("truthy");
        }
      "#}
      .trim_end()
    );
  }

  #[test]
  fn test_if_statement_number_falsy() {
    let code = indoc! {r#"
      if (0) {
        console.log("truthy");
      }
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(output_code, "{}");
  }

  #[test]
  fn test_simple_nested_not() {
    // Test double negation
    let code = indoc! {r#"
      const a = !true;
      const b = !false;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = true;
      "#}
    );
  }

  #[test]
  fn test_complex_nested_not() {
    // Test double negation
    let code = indoc! {r#"
      const a = !!!true;
      const b = !!!false;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = false;
        const b = true;
      "#}
    );
  }

  #[test]
  fn test_binary_and_operator() {
    let code = indoc! {r#"
      const a = true && true;
      const b = true && false;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = true;
        const b = false;
      "#}
    );
  }

  #[test]
  fn test_ternary_nested() {
    let code = indoc! {r#"
      const a = false ? "a" : "b";
      const b = true ? "yes" : "no";
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    assert_eq!(
      output_code,
      indoc! {r#"
        const a = "b";
        const b = "yes";
      "#}
    );
  }

  #[test]
  fn test_no_transformation_with_variables() {
    let code = indoc! {r#"
      const a = x === 1;
      const b = !unknownVar;
      const c = foo && bar;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |_| StaticPreEvaluator);

    // Should remain unchanged
    assert_eq!(
      output_code,
      indoc! {r#"
        const a = x === 1;
        const b = !unknownVar;
        const c = foo && bar;
      "#}
    );
  }
}
