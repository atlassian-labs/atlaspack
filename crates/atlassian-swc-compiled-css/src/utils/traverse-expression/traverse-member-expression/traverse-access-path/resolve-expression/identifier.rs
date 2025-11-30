use swc_core::ecma::ast::{Expr, Ident};

use crate::types::Metadata;
use crate::utils_create_result_pair::{create_result_pair, ResultPair};
use crate::utils_resolve_binding::resolve_binding;
use crate::utils_types::EvaluateExpression;

pub fn evaluate_identifier(
  expression: &Ident,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  if let Some(binding) = resolve_binding(expression.sym.as_ref(), meta.clone(), evaluate_expression)
  {
    if binding.constant {
      if let Some(node) = binding.node.as_ref() {
        let result = (evaluate_expression)(node, binding.meta.clone());
        return create_result_pair(result.value, result.meta);
      }
    }
  }

  create_result_pair(Expr::Ident(expression.clone()), meta)
}
