use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  ArrowExpr, BlockStmt, BlockStmtOrExpr, Expr, FnExpr, Function, Ident, ReturnStmt, Stmt,
};
use swc_core::ecma::visit::{Visit, VisitWith};

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_types::EvaluateExpression;

fn undefined_ident() -> Expr {
  Expr::Ident(Ident::new(
    "undefined".into(),
    DUMMY_SP,
    SyntaxContext::empty(),
  ))
}

/// Checks if a block contains any control flow statements that could result
/// in different return values depending on runtime conditions.
fn has_control_flow(block: &BlockStmt) -> bool {
  block.stmts.iter().any(|stmt| {
    matches!(
      stmt,
      Stmt::If(_)
        | Stmt::Switch(_)
        | Stmt::For(_)
        | Stmt::ForIn(_)
        | Stmt::ForOf(_)
        | Stmt::While(_)
        | Stmt::DoWhile(_)
    )
  })
}

struct ReturnFinder {
  evaluate_expression: EvaluateExpression,
  meta: Metadata,
  result: Option<ResultPair>,
}

impl ReturnFinder {
  fn new(evaluate_expression: EvaluateExpression, meta: Metadata) -> Self {
    Self {
      evaluate_expression,
      meta,
      result: None,
    }
  }

  fn find(
    block: &BlockStmt,
    evaluate_expression: EvaluateExpression,
    meta: Metadata,
  ) -> Option<ResultPair> {
    let mut finder = Self::new(evaluate_expression, meta);
    block.visit_with(&mut finder);
    finder.result
  }
}

impl Visit for ReturnFinder {
  fn visit_return_stmt(&mut self, node: &ReturnStmt) {
    if self.result.is_some() {
      return;
    }

    if let Some(arg) = &node.arg {
      let pair = (self.evaluate_expression)(arg, self.meta.clone());
      self.result = Some(pair);
    } else {
      self.result = Some(create_result_pair(undefined_ident(), self.meta.clone()));
    }
  }

  fn visit_function(&mut self, _: &Function) {}

  fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}
}

fn evaluate_block(
  block: &BlockStmt,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  ReturnFinder::find(block, evaluate_expression, meta.clone())
    .unwrap_or_else(|| create_result_pair(undefined_ident(), meta))
}

fn evaluate_block_with_fallback(
  block: &BlockStmt,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
  fallback: &Expr,
) -> ResultPair {
  // If the block has control flow, we can't statically determine the return value
  // since different branches may return different values. Return the original
  // expression so it can be processed as a dynamic value.
  if has_control_flow(block) {
    return create_result_pair(fallback.clone(), meta);
  }

  ReturnFinder::find(block, evaluate_expression, meta.clone())
    .unwrap_or_else(|| create_result_pair(undefined_ident(), meta))
}

fn evaluate_arrow(
  arrow: &ArrowExpr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  match arrow.body.as_ref() {
    BlockStmtOrExpr::BlockStmt(block) => {
      let fallback = Expr::Arrow(arrow.clone());
      evaluate_block_with_fallback(block, meta, evaluate_expression, &fallback)
    }
    BlockStmtOrExpr::Expr(expr) => (evaluate_expression)(expr, meta),
  }
}

fn evaluate_function(
  function: &Function,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  if let Some(body) = &function.body {
    evaluate_block(body, meta, evaluate_expression)
  } else {
    create_result_pair(undefined_ident(), meta)
  }
}

/// Mirrors the Babel `traverseFunction` helper by evaluating the return value of a
/// function-like expression and pairing it with the updated metadata produced by
/// recursive evaluation.
pub fn traverse_function(
  expression: &Expr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  match expression {
    Expr::Arrow(arrow) => evaluate_arrow(arrow, meta, evaluate_expression),
    Expr::Fn(FnExpr { function, .. }) => evaluate_function(function, meta, evaluate_expression),
    _ => create_result_pair(undefined_ident(), meta),
  }
}

#[cfg(test)]
mod tests {
  use super::traverse_function;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_create_result_pair::{ResultPair, create_result_pair};
  use crate::utils_types::EvaluateExpression;
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{Expr, Lit, Number, Str};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.tsx".into()).into(), code.to_string());
    let lexer = Lexer::new(
      Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      }),
      Default::default(),
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);
    *parser.parse_expr().expect("parse expression")
  }

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn identity_evaluate(expr: &Expr, meta: Metadata) -> ResultPair {
    create_result_pair(expr.clone(), meta)
  }

  #[test]
  fn evaluates_arrow_expression_body() {
    let expr = parse_expression("() => 10");
    let meta = create_metadata();
    let pair = traverse_function(&expr, meta.clone(), identity_evaluate as EvaluateExpression);

    match pair.value {
      Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(value, 10.0),
      other => panic!("expected numeric literal, found {:?}", other),
    }

    assert_eq!(
      pair.meta.state().file().filename,
      meta.state().file().filename
    );
  }

  #[test]
  fn evaluates_arrow_block_return_statement() {
    let expr = parse_expression("() => { return 'test'; }");
    let meta = create_metadata();
    let pair = traverse_function(&expr, meta, identity_evaluate as EvaluateExpression);

    match pair.value {
      Expr::Lit(Lit::Str(Str { value, .. })) => assert_eq!(value.as_ref(), "test"),
      other => panic!("expected string literal, found {:?}", other),
    }
  }

  #[test]
  fn evaluates_function_expression_return() {
    let expr = parse_expression("function () { return 42; }");
    let meta = create_metadata();
    let pair = traverse_function(&expr, meta, identity_evaluate as EvaluateExpression);

    match pair.value {
      Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(value, 42.0),
      other => panic!("expected numeric literal, found {:?}", other),
    }
  }

  #[test]
  fn ignores_nested_function_returns() {
    let expr = parse_expression("() => { function inner() { return 1; } return 2; }");
    let meta = create_metadata();
    let pair = traverse_function(&expr, meta, identity_evaluate as EvaluateExpression);

    match pair.value {
      Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(value, 2.0),
      other => panic!("expected numeric literal, found {:?}", other),
    }
  }
}
