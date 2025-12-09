use swc_core::common::Spanned;
use swc_core::ecma::ast::{Callee, Expr, ExprOrSpread, MemberExpr};

use crate::types::Metadata;
use crate::utils_create_result_pair::{create_result_pair, ResultPair};
use crate::utils_is_compiled::is_compiled_css_call_expression;
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path_resolve_expression_function_args::get_function_args;
use crate::utils_traverse_expression_traverse_member_expression_traverse_access_path_resolve_expression_identifier::evaluate_identifier;
use crate::utils_types::EvaluateExpression;

fn expressions_are_identical(a: &Expr, b: &Expr) -> bool {
  std::mem::discriminant(a) == std::mem::discriminant(b) && a.span() == b.span()
}

fn call_function_expression(function: &Expr, args: Vec<ExprOrSpread>) -> Expr {
  Expr::Call(swc_core::ecma::ast::CallExpr {
    span: function.span(),
    ctxt: Default::default(),
    callee: Callee::Expr(Box::new(function.clone())),
    args,
    type_args: None,
  })
}

fn resolve_function_expression(
  expression: &Expr,
  meta: Metadata,
  expression_name: &str,
  member_expression: &MemberExpr,
  call_arguments: Option<&[ExprOrSpread]>,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  let args = get_function_args(expression_name, member_expression, call_arguments);
  let call_expression = call_function_expression(expression, args);
  (evaluate_expression)(&call_expression, meta)
}

fn resolve_compiled_css_call(
  expression: &Expr,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> Option<ResultPair> {
  if let Expr::Call(call) = expression {
    if let Some(arg) = call.args.get(0) {
      return Some((evaluate_expression)(&arg.expr, meta));
    }
  }

  None
}

pub fn resolve_expression_in_member(
  expression: &Expr,
  meta: Metadata,
  expression_name: &str,
  member_expression: &MemberExpr,
  call_arguments: Option<&[ExprOrSpread]>,
  evaluate_expression: EvaluateExpression,
) -> ResultPair {
  let mut current_expression = expression.clone();
  let mut current_meta = meta;

  loop {
    let mut result = create_result_pair(current_expression.clone(), current_meta.clone());

    match &current_expression {
      Expr::Ident(ident) => {
        result = evaluate_identifier(ident, current_meta.clone(), evaluate_expression);
      }
      Expr::Fn(_) | Expr::Arrow(_) => {
        result = resolve_function_expression(
          &current_expression,
          current_meta.clone(),
          expression_name,
          member_expression,
          call_arguments,
          evaluate_expression,
        );
      }
      Expr::Call(_) => {
        let is_compiled = {
          let state = current_meta.state();
          is_compiled_css_call_expression(&current_expression, &state)
        };

        if is_compiled {
          if let Some(resolved) = resolve_compiled_css_call(
            &current_expression,
            current_meta.clone(),
            evaluate_expression,
          ) {
            result = resolved;
          } else {
            result = (evaluate_expression)(&current_expression, current_meta.clone());
          }
        } else {
          result = (evaluate_expression)(&current_expression, current_meta.clone());
        }
      }
      Expr::Member(_) => {
        result = (evaluate_expression)(&current_expression, current_meta.clone());
      }
      _ => {
        let is_compiled = {
          let state = current_meta.state();
          is_compiled_css_call_expression(&current_expression, &state)
        };

        if is_compiled {
          if let Some(resolved) = resolve_compiled_css_call(
            &current_expression,
            current_meta.clone(),
            evaluate_expression,
          ) {
            result = resolved;
          }
        }
      }
    }

    if expressions_are_identical(&result.value, &current_expression) {
      return result;
    }

    current_expression = result.value.clone();
    current_meta = result.meta.clone();
  }
}
