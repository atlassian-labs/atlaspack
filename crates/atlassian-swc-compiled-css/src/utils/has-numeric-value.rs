use swc_core::ecma::ast::{Expr, Lit};

/// Determine whether the provided expression represents a numeric value.
///
/// Mirrors the Babel helper by returning `true` for numeric literals and for
/// string literals that coerce to a finite JavaScript number (using the same
/// semantics as `Number(value)`).
pub fn has_numeric_value(expr: &Expr) -> bool {
  match expr {
    Expr::Lit(Lit::Num(_)) => true,
    Expr::Lit(Lit::Str(str_lit)) => string_has_numeric_value(str_lit.value.as_ref()),
    _ => false,
  }
}

fn string_has_numeric_value(value: &str) -> bool {
  let trimmed = value.trim();

  if trimmed.is_empty() {
    return true;
  }

  if trimmed == "NaN" {
    return false;
  }

  if matches!(trimmed, "Infinity" | "+Infinity" | "-Infinity") {
    return true;
  }

  if is_prefixed_radix(trimmed, "0x", 16) || is_prefixed_radix(trimmed, "0X", 16) {
    return true;
  }

  if is_prefixed_radix(trimmed, "0b", 2) || is_prefixed_radix(trimmed, "0B", 2) {
    return true;
  }

  if is_prefixed_radix(trimmed, "0o", 8) || is_prefixed_radix(trimmed, "0O", 8) {
    return true;
  }

  is_decimal_numeric(trimmed)
}

fn is_prefixed_radix(value: &str, prefix: &str, radix: u32) -> bool {
  if !value.starts_with(prefix) {
    return false;
  }

  let digits = &value[prefix.len()..];
  !digits.is_empty() && digits.chars().all(|ch| ch.is_digit(radix))
}

fn is_decimal_numeric(value: &str) -> bool {
  let mut chars = value.chars().peekable();

  if let Some(&sign) = chars.peek() {
    if sign == '+' || sign == '-' {
      chars.next();
    }
  }

  let mut digits_before_decimal = 0;
  let mut digits_after_decimal = 0;
  let mut digits_after_exponent = 0;
  let mut seen_decimal = false;
  let mut seen_exponent = false;

  while let Some(ch) = chars.next() {
    match ch {
      '0'..='9' => {
        if seen_exponent {
          digits_after_exponent += 1;
        } else if seen_decimal {
          digits_after_decimal += 1;
        } else {
          digits_before_decimal += 1;
        }
      }
      '.' if !seen_decimal && !seen_exponent => {
        seen_decimal = true;
      }
      'e' | 'E' if !seen_exponent && (digits_before_decimal > 0 || digits_after_decimal > 0) => {
        seen_exponent = true;

        if let Some(&next) = chars.peek() {
          if next == '+' || next == '-' {
            chars.next();
          }
        }

        match chars.peek() {
          Some(digit) if digit.is_ascii_digit() => {}
          _ => return false,
        }
      }
      _ => return false,
    }
  }

  if seen_exponent {
    if digits_after_exponent == 0 {
      return false;
    }
    digits_before_decimal > 0 || digits_after_decimal > 0
  } else {
    digits_before_decimal > 0 || digits_after_decimal > 0
  }
}

#[cfg(test)]
mod tests {
  use swc_core::common::DUMMY_SP;
  use swc_core::ecma::ast::{Expr, Lit, Number, Str};

  use super::has_numeric_value;

  fn numeric(value: f64) -> Expr {
    Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    }))
  }

  fn string(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  #[test]
  fn matches_numeric_literals() {
    assert!(has_numeric_value(&numeric(10.0)));
  }

  #[test]
  fn matches_decimal_strings() {
    assert!(has_numeric_value(&string("42")));
    assert!(has_numeric_value(&string("  -0.5e2  ")));
    assert!(has_numeric_value(&string(".5")));
  }

  #[test]
  fn matches_prefixed_numbers() {
    assert!(has_numeric_value(&string("0x1f")));
    assert!(has_numeric_value(&string("0b10")));
    assert!(has_numeric_value(&string("0O7")));
  }

  #[test]
  fn matches_infinity_variants() {
    assert!(has_numeric_value(&string("Infinity")));
    assert!(has_numeric_value(&string("-Infinity")));
    assert!(has_numeric_value(&string("+Infinity")));
  }

  #[test]
  fn rejects_nan_and_invalid_numbers() {
    assert!(!has_numeric_value(&string("NaN")));
    assert!(!has_numeric_value(&string("foo")));
    assert!(!has_numeric_value(&string("0x")));
    assert!(!has_numeric_value(&string("0b102")));
    assert!(!has_numeric_value(&string("-.")));
    assert!(!has_numeric_value(&string("+")));
  }

  #[test]
  fn accepts_empty_string() {
    assert!(has_numeric_value(&string("")));
    assert!(has_numeric_value(&string("   ")));
  }
}
