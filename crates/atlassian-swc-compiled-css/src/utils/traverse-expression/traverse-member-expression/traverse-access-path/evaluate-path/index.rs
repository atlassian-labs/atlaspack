use swc_core::ecma::ast::{Expr, TsAsExpr};

use crate::types::Metadata;
use crate::utils_create_result_pair::{create_result_pair, ResultPair};
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path_evaluate_path_namespace_import::evaluate_namespace_import_path;
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path_evaluate_path_object::evaluate_object_path;
use crate::utils_types::EvaluateExpression;

fn unwrap_ts_as_expression(expression: &TsAsExpr) -> &Expr {
  &expression.expr
}

pub fn evaluate_path(
  expression: &Expr,
  meta: Metadata,
  path_name: &str,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  match expression {
    Expr::Object(object) => evaluate_object_path(object, meta, path_name),
    Expr::TsAs(ts_as) => evaluate_path(
      unwrap_ts_as_expression(ts_as),
      meta,
      path_name,
      evaluate_expression,
    ),
    Expr::TsConstAssertion(assertion) => {
      evaluate_path(&assertion.expr, meta, path_name, evaluate_expression)
    }
    Expr::TsTypeAssertion(assertion) => {
      evaluate_path(&assertion.expr, meta, path_name, evaluate_expression)
    }
    Expr::TsNonNull(assertion) => {
      evaluate_path(&assertion.expr, meta, path_name, evaluate_expression)
    }
    Expr::Paren(paren) => evaluate_path(&paren.expr, meta, path_name, evaluate_expression),
    Expr::Ident(_) => {
      evaluate_namespace_import_path(expression, meta.clone(), path_name, evaluate_expression)
        .unwrap_or_else(|| create_result_pair(expression.clone(), meta))
    }
    _ => create_result_pair(expression.clone(), meta),
  }
}
