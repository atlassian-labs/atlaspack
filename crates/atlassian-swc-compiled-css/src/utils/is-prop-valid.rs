use std::collections::HashSet;

use once_cell::sync::Lazy;

use crate::utils_is_prop_valid_data::ALLOWED_PROPS;

static ALLOWED_PROPS_SET: Lazy<HashSet<&'static str>> =
  Lazy::new(|| ALLOWED_PROPS.iter().copied().collect());

fn is_ascii_match(byte: u8, lower: u8, upper: u8) -> bool {
  byte == lower || byte == upper
}

fn has_prefix(prop: &str, prefix: &[u8]) -> bool {
  if prop.len() <= prefix.len() || !prop.is_ascii() {
    return false;
  }

  let bytes = prop.as_bytes();

  for (index, expected) in prefix.iter().enumerate() {
    if !is_ascii_match(
      bytes[index],
      expected.to_ascii_lowercase(),
      expected.to_ascii_uppercase(),
    ) {
      return false;
    }
  }

  bytes[prefix.len()] == b'-'
}

fn is_data_attribute(prop: &str) -> bool {
  has_prefix(prop, b"data")
}

fn is_aria_attribute(prop: &str) -> bool {
  has_prefix(prop, b"aria")
}

fn is_prefixed_attribute(prop: &str) -> bool {
  if prop.len() < 2 {
    return false;
  }

  if is_data_attribute(prop) || is_aria_attribute(prop) {
    return true;
  }

  let bytes = prop.as_bytes();
  bytes[0] == b'x' && bytes[1] == b'-'
}

fn is_event_handler(prop: &str) -> bool {
  let bytes = prop.as_bytes();
  if bytes.len() < 3 {
    return false;
  }

  bytes[0] == b'o' && bytes[1] == b'n' && bytes[2].is_ascii_uppercase()
}

/// Mirrors `@emotion/is-prop-valid` by validating React DOM props, allowing
/// `data-*`, `aria-*`, `x-*`, and `on*` event handler attributes.
pub fn is_prop_valid(prop: &str) -> bool {
  if prop.is_empty() {
    return false;
  }

  ALLOWED_PROPS_SET.contains(prop) || is_prefixed_attribute(prop) || is_event_handler(prop)
}

#[cfg(test)]
mod tests {
  use super::is_prop_valid;

  #[test]
  fn allows_known_dom_props() {
    assert!(is_prop_valid("children"));
    assert!(is_prop_valid("className"));
  }

  #[test]
  fn allows_data_and_aria_attributes() {
    assert!(is_prop_valid("data-test-id"));
    assert!(is_prop_valid("ARIA-label"));
    assert!(is_prop_valid("x-some-prop"));
  }

  #[test]
  fn allows_event_handlers() {
    assert!(is_prop_valid("onClick"));
    assert!(is_prop_valid("onMouseEnter"));
  }

  #[test]
  fn rejects_unknown_props() {
    assert!(!is_prop_valid("textSize"));
    assert!(!is_prop_valid("onlower"));
  }
}
