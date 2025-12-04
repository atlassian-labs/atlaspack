use swc_core::ecma::ast::{CallExpr, Callee, Expr, Ident, MemberExpr, MemberProp, SeqExpr};

/// Returns `true` when the provided call expression resembles an automatic
/// runtime helper such as `_jsx()` or `_jsxs()`.
pub fn is_automatic_runtime(call: &CallExpr, func: &str) -> bool {
  match &call.callee {
    Callee::Expr(expr) => match &**expr {
      Expr::Ident(ident) => is_helper_ident(ident, func),
      Expr::Paren(paren) => is_paren_helper(paren, func),
      Expr::Seq(seq) => sequence_targets_helper(seq, func),
      _ => false,
    },
    Callee::Super(_) | Callee::Import(_) => false,
  }
}

fn is_helper_ident(ident: &Ident, func: &str) -> bool {
  ident.sym.as_ref() == format!("_{}", func)
}

fn is_paren_helper(paren: &swc_core::ecma::ast::ParenExpr, func: &str) -> bool {
  match &*paren.expr {
    Expr::Ident(ident) => is_helper_ident(ident, func),
    Expr::Seq(seq) => sequence_targets_helper(seq, func),
    Expr::Paren(inner) => is_paren_helper(inner, func),
    _ => false,
  }
}

fn sequence_targets_helper(seq: &SeqExpr, func: &str) -> bool {
  if seq.exprs.len() < 2 {
    return false;
  }

  match &*seq.exprs[1] {
    Expr::Member(MemberExpr {
      prop: MemberProp::Ident(property),
      ..
    }) => property.sym.as_ref() == func,
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use swc_core::common::{DUMMY_SP, SyntaxContext};
  use swc_core::ecma::ast::{CallExpr, Expr, Ident, Lit, MemberExpr, MemberProp, Number, SeqExpr};

  fn ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
  }

  fn call_with_callee(callee: Expr) -> CallExpr {
    CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(callee)),
      args: Vec::new(),
      type_args: None,
    }
  }

  #[test]
  fn detects_helper_ident() {
    let call = call_with_callee(Expr::Ident(ident("_jsxs")));
    assert!(is_automatic_runtime(&call, "jsxs"));
    assert!(!is_automatic_runtime(&call, "jsx"));
  }

  #[test]
  fn detects_sequence_expression() {
    let member = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 0.0,
        raw: None,
      }))),
      prop: MemberProp::Ident(ident("jsx").into()),
    });
    let seq = Expr::Seq(SeqExpr {
      span: DUMMY_SP,
      exprs: vec![
        Box::new(Expr::Lit(Lit::Num(Number {
          span: DUMMY_SP,
          value: 0.0,
          raw: None,
        }))),
        Box::new(member),
      ],
    });

    let call = call_with_callee(seq);
    assert!(is_automatic_runtime(&call, "jsx"));
  }

  #[test]
  fn detects_parenthesized_sequence_expression() {
    let member = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(ident("_jsxRuntime"))),
      prop: MemberProp::Ident(ident("jsxs").into()),
    });
    let seq = Expr::Seq(SeqExpr {
      span: DUMMY_SP,
      exprs: vec![
        Box::new(Expr::Lit(Lit::Num(Number {
          span: DUMMY_SP,
          value: 0.0,
          raw: None,
        }))),
        Box::new(member),
      ],
    });
    let paren = Expr::Paren(swc_core::ecma::ast::ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(seq),
    });

    let call = call_with_callee(paren);
    assert!(is_automatic_runtime(&call, "jsxs"));
  }

  #[test]
  fn rejects_other_calls() {
    let call = call_with_callee(Expr::Ident(ident("something_else")));
    assert!(!is_automatic_runtime(&call, "jsx"));
  }
}
