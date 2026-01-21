/// Utilities for working with vendor-prefixed CSS identifiers.
///
/// These functions match the behaviour of PostCSS 8.4.31's `vendor` helpers.
#[inline]
pub fn prefix(prop: &str) -> String {
  let mut chars = prop.chars();
  match (chars.next(), chars.next()) {
    (Some('-'), Some(second)) if second != '-' => {
      if let Some(rel_index) = prop[1..].find('-') {
        let end = rel_index + 1 + 1; // include the second dash
        prop[..end].to_string()
      } else {
        String::new()
      }
    }
    _ => String::new(),
  }
}

#[inline]
pub fn unprefixed(prop: &str) -> String {
  let mut chars = prop.chars();
  match (chars.next(), chars.next()) {
    (Some('-'), Some(second)) if second != '-' => {
      if let Some(rel_index) = prop[1..].find('-') {
        prop[rel_index + 1 + 1..].to_string()
      } else {
        prop.to_string()
      }
    }
    _ => prop.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::{prefix, unprefixed};

  #[test]
  fn prefix_extracts_vendor_segment() {
    assert_eq!(prefix("-webkit-border-radius"), "-webkit-");
    assert_eq!(prefix("-moz-transition"), "-moz-");
  }

  #[test]
  fn prefix_returns_empty_when_no_vendor() {
    assert_eq!(prefix("color"), "");
    assert_eq!(prefix("--custom-prop"), "");
    assert_eq!(prefix("-o"), "");
  }

  #[test]
  fn unprefixed_strips_vendor_prefix() {
    assert_eq!(unprefixed("-webkit-border-radius"), "border-radius");
    assert_eq!(unprefixed("-moz-transition"), "transition");
  }

  #[test]
  fn unprefixed_returns_original_for_custom_props() {
    assert_eq!(unprefixed("--custom-prop"), "--custom-prop");
    assert_eq!(unprefixed("color"), "color");
    assert_eq!(unprefixed("-o"), "-o");
  }
}
