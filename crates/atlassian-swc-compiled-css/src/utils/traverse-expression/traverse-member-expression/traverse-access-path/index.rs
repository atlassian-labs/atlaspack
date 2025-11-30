use swc_core::ecma::ast::{Expr, ExprOrSpread, Ident, MemberExpr};

use crate::types::Metadata;
use crate::utils_create_result_pair::ResultPair;
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path_evaluate_path::evaluate_path;
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path_resolve_expression::resolve_expression_in_member;
use crate::utils_types::EvaluateExpression;

pub fn traverse_member_access_path(
  expression: &Expr,
  meta: Metadata,
  expression_name: &str,
  access_path: &[Ident],
  member_expression: &MemberExpr,
  call_arguments: Option<&[ExprOrSpread]>,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  if std::env::var("COMPILED_TRACE_MEMBER_PATH").is_ok() {
    let path: Vec<String> = access_path.iter().map(|id| id.sym.to_string()).collect();
    eprintln!(
      "[compiled][member-path] expr={} access_path={:?} span={:?}",
      expression_name,
      path,
      member_expression.span
    );
  }

  let result = resolve_expression_in_member(
    expression,
    meta,
    expression_name,
    member_expression,
    call_arguments,
    evaluate_expression,
  );

  if let Some((segment, remaining)) = access_path.split_first() {
    let evaluated = evaluate_path(
      &result.value,
      result.meta.clone(),
      segment.sym.as_ref(),
      evaluate_expression,
    );
    return traverse_member_access_path(
      &evaluated.value,
      evaluated.meta,
      segment.sym.as_ref(),
      remaining,
      member_expression,
      call_arguments,
      evaluate_expression,
    );
  }

  result
}
