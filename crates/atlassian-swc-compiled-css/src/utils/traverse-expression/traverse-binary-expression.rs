use swc_core::ecma::ast::{BinExpr, Expr, PrivateName};

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_has_numeric_value::has_numeric_value;
use crate::utils_types::EvaluateExpression;

fn is_private_name(expr: &Expr) -> bool {
  matches!(expr, Expr::PrivateName(PrivateName { .. }))
}

/// Mirrors the Babel `traverseBinaryExpression` helper by recursively
/// evaluating both sides of the binary expression and reusing the evaluated
/// operands when they both represent numeric values.
pub fn traverse_binary_expression(
  expression: &BinExpr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  if !is_private_name(expression.left.as_ref()) {
    let left_pair = (evaluate_expression)(expression.left.as_ref(), meta.clone());
    let right_pair = (evaluate_expression)(expression.right.as_ref(), meta.clone());

    if has_numeric_value(&left_pair.value) && has_numeric_value(&right_pair.value) {
      let evaluated = Expr::Bin(BinExpr {
        span: expression.span,
        op: expression.op,
        left: Box::new(left_pair.value),
        right: Box::new(right_pair.value),
      });

      return create_result_pair(evaluated, meta);
    }
  }

  create_result_pair(Expr::Bin(expression.clone()), meta)
}

#[cfg(test)]
mod tests {
  use super::traverse_binary_expression;
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_create_result_pair::{ResultPair, create_result_pair};
  use crate::utils_types::EvaluateExpression;
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::atoms::Atom;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, FileName, SourceMap};
  use swc_core::ecma::ast::{BinExpr, BinaryOp, Expr, Lit, Number, PrivateName, Str};
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

  fn evaluate_numeric(expr: &Expr, meta: Metadata) -> ResultPair {
    match expr {
      Expr::Ident(ident) if ident.sym.as_ref() == "a" => create_result_pair(numeric(1.0), meta),
      Expr::Ident(ident) if ident.sym.as_ref() == "b" => create_result_pair(numeric(2.0), meta),
      _ => create_result_pair(expr.clone(), meta),
    }
  }

  fn evaluate_non_numeric(expr: &Expr, meta: Metadata) -> ResultPair {
    match expr {
      Expr::Ident(ident) if ident.sym.as_ref() == "a" => create_result_pair(
        Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "value".into(),
          raw: None,
        })),
        meta,
      ),
      _ => create_result_pair(expr.clone(), meta),
    }
  }

  #[test]
  fn replaces_identifier_operands_with_evaluated_values() {
    let expr = parse_expression("a + b");
    let meta = create_metadata();

    let bin_expr = match expr {
      Expr::Bin(bin) => bin,
      other => panic!("expected binary expression, found {:?}", other),
    };

    let pair = traverse_binary_expression(
      &bin_expr,
      meta.clone(),
      evaluate_numeric as EvaluateExpression,
    );

    match pair.value {
      Expr::Bin(BinExpr {
        op, left, right, ..
      }) => {
        assert_eq!(op, BinaryOp::Add);

        match left.as_ref() {
          Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(*value, 1.0),
          other => panic!("expected numeric literal, found {:?}", other),
        }

        match right.as_ref() {
          Expr::Lit(Lit::Num(Number { value, .. })) => assert_eq!(*value, 2.0),
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
  fn returns_original_expression_when_operands_are_not_numeric() {
    let expr = parse_expression("a + b");
    let meta = create_metadata();

    let bin_expr = match expr {
      Expr::Bin(bin) => bin,
      other => panic!("expected binary expression, found {:?}", other),
    };

    let pair = traverse_binary_expression(
      &bin_expr,
      meta.clone(),
      evaluate_non_numeric as EvaluateExpression,
    );

    assert_eq!(pair.value, Expr::Bin(bin_expr.clone()));
  }

  #[test]
  fn skips_private_name_operands() {
    let private = Expr::PrivateName(PrivateName {
      span: DUMMY_SP,
      name: Atom::from("secret"),
    });

    let expr = BinExpr {
      span: DUMMY_SP,
      op: BinaryOp::Add,
      left: Box::new(private.clone()),
      right: Box::new(numeric(10.0)),
    };

    let meta = create_metadata();
    let pair =
      traverse_binary_expression(&expr, meta.clone(), evaluate_numeric as EvaluateExpression);

    assert_eq!(pair.value, Expr::Bin(expr.clone()));
  }
}
