use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{BinExpr, BinaryOp, Expr, Lit, Number, UnaryExpr, UnaryOp};

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_has_numeric_value::has_numeric_value;
use crate::utils_types::EvaluateExpression;

fn numeric_literal(value: f64) -> Expr {
  Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value,
    raw: None,
  }))
}

/// Mirrors the Babel `traverseUnaryExpression` helper by normalising unary
/// negation of non-numeric values into a `-1 * value` binary expression so the
/// downstream evaluation pipeline can continue resolving the argument.
pub fn traverse_unary_expression(
  expression: &UnaryExpr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  if expression.op == UnaryOp::Minus && !has_numeric_value(&expression.arg) {
    let evaluated_argument = (evaluate_expression)(expression.arg.as_ref(), meta.clone()).value;
    let binary = Expr::Bin(BinExpr {
      span: expression.span,
      op: BinaryOp::Mul,
      left: Box::new(numeric_literal(-1.0)),
      right: Box::new(evaluated_argument),
    });

    return create_result_pair(binary, meta);
  }

  create_result_pair(Expr::Unary(expression.clone()), meta)
}

#[cfg(test)]
mod tests {
  use super::traverse_unary_expression;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_create_result_pair::{ResultPair, create_result_pair};
  use crate::utils_types::EvaluateExpression;
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, FileName, SourceMap};
  use swc_core::ecma::ast::{BinExpr, BinaryOp, Expr, Lit, Number};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn parse_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.tsx".into()).into(), code.into());
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

  fn numeric(value: f64) -> Expr {
    Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))
  }

  fn evaluate_identifier(expr: &Expr, meta: Metadata) -> ResultPair {
    match expr {
      Expr::Ident(ident) if ident.sym.as_ref() == "foo" => create_result_pair(numeric(5.0), meta),
      _ => create_result_pair(expr.clone(), meta),
    }
  }

  #[test]
  fn converts_negative_identifier_to_binary_expression() {
    let expr = parse_expression("-foo");
    let meta = create_metadata();

    let unary = match expr {
      Expr::Unary(unary) => unary,
      other => panic!("expected unary expression, found {:?}", other),
    };

    let pair = traverse_unary_expression(
      &unary,
      meta.clone(),
      evaluate_identifier as EvaluateExpression,
    );

    match pair.value {
      Expr::Bin(BinExpr {
        op, left, right, ..
      }) => {
        assert_eq!(op, BinaryOp::Mul);

        match left.as_ref() {
          Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(*value, -1.0),
          other => panic!("expected numeric literal, found {:?}", other),
        }

        match right.as_ref() {
          Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(*value, 5.0),
          other => panic!("expected numeric literal, found {:?}", other),
        }
      }
      other => panic!("expected binary expression, found {:?}", other),
    }

    assert_eq!(
      pair.meta.state().file().filename,
      meta.state().file().filename
    );
  }

  #[test]
  fn preserves_numeric_literal_negation() {
    let expr = parse_expression("-8");
    let meta = create_metadata();

    let unary = match expr {
      Expr::Unary(unary) => unary,
      other => panic!("expected unary expression, found {:?}", other),
    };

    let pair = traverse_unary_expression(&unary, meta.clone(), evaluate_identifier);

    assert_eq!(pair.value, Expr::Unary(unary.clone()));
  }
}
