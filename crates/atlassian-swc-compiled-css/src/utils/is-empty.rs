use swc_core::ecma::ast::{Expr, Ident, Lit};

/// Mirrors the Babel helper by determining if an expression represents an empty
/// value – `undefined`, `null`, or an empty string literal.
pub fn is_empty_value(expr: &Expr) -> bool {
  match expr {
    Expr::Ident(Ident { sym, .. }) => sym.as_ref() == "undefined",
    Expr::Lit(Lit::Null(_)) => true,
    Expr::Lit(Lit::Str(str_lit)) => str_lit.value.is_empty(),
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use swc_core::common::{DUMMY_SP, SyntaxContext};
  use swc_core::ecma::ast::{Expr, Ident, Lit, Null, Str};

  use super::is_empty_value;

  fn ident(name: &str) -> Expr {
    Expr::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()))
  }

  fn null_literal() -> Expr {
    Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
  }

  fn string_literal(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  #[test]
  fn detects_undefined_identifier() {
    assert!(is_empty_value(&ident("undefined")));
  }

  #[test]
  fn detects_null_literal() {
    assert!(is_empty_value(&null_literal()));
  }

  #[test]
  fn detects_empty_string_literal() {
    assert!(is_empty_value(&string_literal("")));
  }

  #[test]
  fn rejects_other_values() {
    assert!(!is_empty_value(&ident("value")));
    assert!(!is_empty_value(&string_literal("text")));
  }
}
