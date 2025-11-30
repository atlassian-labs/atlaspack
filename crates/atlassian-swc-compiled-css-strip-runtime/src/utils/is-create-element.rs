use swc_core::ecma::ast::{Expr, MemberExpr, MemberProp};

/// Returns `true` when the expression resembles `React.createElement`.
pub fn is_create_element(expr: &Expr) -> bool {
  match expr {
    Expr::Member(MemberExpr { obj, prop, .. }) => match (&**obj, prop) {
      (Expr::Ident(object), MemberProp::Ident(property)) => {
        object.sym.as_ref() == "React" && property.sym.as_ref() == "createElement"
      }
      _ => false,
    },
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use swc_core::common::{SyntaxContext, DUMMY_SP};
  use swc_core::ecma::ast::{Ident, Lit, MemberProp, Null};

  fn ident(name: &str) -> Ident {
    Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty())
  }

  #[test]
  fn matches_react_create_element() {
    let expr = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(ident("React"))),
      prop: MemberProp::Ident(ident("createElement").into()),
    });

    assert!(is_create_element(&expr));
  }

  #[test]
  fn rejects_other_members() {
    let expr = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(ident("React"))),
      prop: MemberProp::Ident(ident("jsx").into()),
    });

    assert!(!is_create_element(&expr));
  }

  #[test]
  fn rejects_non_members() {
    let expr = Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
    assert!(!is_create_element(&expr));
  }
}
