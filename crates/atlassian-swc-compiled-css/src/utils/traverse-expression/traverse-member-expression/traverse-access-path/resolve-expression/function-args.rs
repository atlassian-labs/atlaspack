use swc_core::ecma::ast::{Callee, Expr, ExprOrSpread, MemberExpr, MemberProp, SeqExpr};

fn matches_function_name(expr: &Expr, function_name: &str) -> bool {
  match expr {
    Expr::Ident(ident) => ident.sym.as_ref() == function_name,
    Expr::Member(member) => match &member.prop {
      MemberProp::Ident(ident) => ident.sym.as_ref() == function_name,
      _ => false,
    },
    _ => false,
  }
}

fn find_call_arguments(expr: &Expr, function_name: &str) -> Option<Vec<ExprOrSpread>> {
  match expr {
    Expr::Call(call) => {
      let matches = match &call.callee {
        Callee::Expr(callee_expr) => matches_function_name(callee_expr, function_name),
        _ => false,
      };

      if matches {
        return Some(call.args.clone());
      }

      if let Callee::Expr(callee_expr) = &call.callee {
        if let Some(args) = find_call_arguments(callee_expr, function_name) {
          return Some(args);
        }
      }

      for arg in &call.args {
        if let Some(args) = find_call_arguments(&arg.expr, function_name) {
          return Some(args);
        }
      }
    }
    Expr::Member(member) => {
      if let Some(args) = find_call_arguments(&member.obj, function_name) {
        return Some(args);
      }

      if let MemberProp::Computed(computed) = &member.prop {
        if let Some(args) = find_call_arguments(&computed.expr, function_name) {
          return Some(args);
        }
      }
    }
    Expr::Paren(paren) => {
      return find_call_arguments(&paren.expr, function_name);
    }
    Expr::Seq(SeqExpr { exprs, .. }) => {
      if let Some(last) = exprs.last() {
        return find_call_arguments(last, function_name);
      }
    }
    _ => {}
  }

  None
}

pub fn get_function_args(
  function_name: &str,
  member_expression: &MemberExpr,
  call_arguments: Option<&[ExprOrSpread]>,
) -> Vec<ExprOrSpread> {
  if let Some(arguments) = call_arguments {
    return arguments.iter().cloned().collect();
  }

  let member_expr = Expr::Member(member_expression.clone());

  find_call_arguments(&member_expr, function_name).unwrap_or_default()
}
