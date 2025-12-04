use swc_core::ecma::ast::{Expr, Ident, MemberProp};

/// Returns `true` when the provided expression references the compiled
/// component container (`CC`).
pub fn is_cc_component(expr: &Expr) -> bool {
  match expr {
    Expr::Ident(ident) => is_cc_ident(ident),
    Expr::Member(member) => match &member.prop {
      MemberProp::Ident(property) => property.sym.as_ref() == "CC",
      _ => false,
    },
    _ => false,
  }
}

fn is_cc_ident(ident: &Ident) -> bool {
  ident.sym.as_ref() == "CC"
}

#[cfg(test)]
mod tests {
  use super::*;
  use swc_core::common::{DUMMY_SP, SyntaxContext};
  use swc_core::ecma::ast::{Ident, Lit, MemberExpr, MemberProp, Null};

  fn ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
  }

  #[test]
  fn matches_identifier() {
    let expr = Expr::Ident(ident("CC"));
    assert!(is_cc_component(&expr));
  }

  #[test]
  fn matches_member_property() {
    let member = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
      prop: MemberProp::Ident(ident("CC").into()),
    });

    assert!(is_cc_component(&member));
  }

  #[test]
  fn rejects_other_values() {
    assert!(!is_cc_component(&Expr::Ident(ident("CS"))));
  }
}
