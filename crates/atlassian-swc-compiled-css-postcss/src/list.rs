//! Utilities for splitting CSS value lists while respecting PostCSS quirks.
//!
//! This module mirrors the behaviour of `lib/list.js` from the JavaScript
//! distribution. It exposes helpers for splitting comma- and space-separated
//! lists while keeping quoted strings, escaped characters, and function
//! parentheses intact.

/// Split a string on commas following PostCSS list semantics.
///
/// Equivalent to `list.comma(string)` in the JavaScript implementation.
pub fn comma(string: &str) -> Vec<String> {
  split(string, &[','], true)
}

/// Split a string on whitespace following PostCSS list semantics.
///
/// Equivalent to `list.space(string)` in the JavaScript implementation.
pub fn space(string: &str) -> Vec<String> {
  split(string, &[' ', '\n', '\t'], false)
}

/// Core list splitting routine used by both [`comma`] and [`space`].
///
/// * `string` - Text to split.
/// * `separators` - Characters that trigger a split when not inside quotes or
///   parentheses.
/// * `last` - When true, behaviour matches PostCSS' `list.split` `last`
///   parameter by adding the last (possibly empty) item even if the string
///   ended with a separator.
pub fn split(string: &str, separators: &[char], last: bool) -> Vec<String> {
  let mut array: Vec<String> = Vec::new();
  let mut current = String::new();
  let mut should_split = false;

  let mut func_level = 0u32;
  let mut in_quote = false;
  let mut prev_quote = '\0';
  let mut escape = false;

  for ch in string.chars() {
    if escape {
      escape = false;
    } else if ch == '\\' {
      escape = true;
    } else if in_quote {
      if ch == prev_quote {
        in_quote = false;
      }
    } else if ch == '"' || ch == '\'' {
      in_quote = true;
      prev_quote = ch;
    } else if ch == '(' {
      func_level = func_level.saturating_add(1);
    } else if ch == ')' {
      func_level = func_level.saturating_sub(1);
    } else if func_level == 0 && separators.contains(&ch) {
      should_split = true;
    }

    if should_split {
      if !current.is_empty() {
        array.push(current.trim().to_string());
      }
      current.clear();
      should_split = false;
    } else {
      current.push(ch);
    }
  }

  if last || !current.is_empty() {
    array.push(current.trim().to_string());
  }

  array
}

#[cfg(test)]
mod tests {
  use super::{comma, space, split};

  #[test]
  fn space_splits_basic() {
    assert_eq!(space("a b"), vec!["a", "b"]);
  }

  #[test]
  fn space_trims_values() {
    assert_eq!(space(" a  b "), vec!["a", "b"]);
  }

  #[test]
  fn space_respects_quotes() {
    assert_eq!(space("\"a b\\\"\" ''"), vec!["\"a b\\\"\"", "''"]);
  }

  #[test]
  fn space_respects_functions() {
    assert_eq!(space("f( )) a( () )"), vec!["f( ))", "a( () )"]);
  }

  #[test]
  fn space_keeps_escaped_spaces() {
    assert_eq!(space("a\\ b"), vec!["a\\ b"]);
  }

  #[test]
  fn comma_splits_basic() {
    assert_eq!(comma("a, b"), vec!["a", "b"]);
  }

  #[test]
  fn comma_adds_last_empty() {
    assert_eq!(comma("a, b,"), vec!["a", "b", ""]);
  }

  #[test]
  fn comma_respects_quotes() {
    assert_eq!(comma("\"a,b\\\"\", ''"), vec!["\"a,b\\\"\"", "''"]);
  }

  #[test]
  fn comma_respects_functions() {
    assert_eq!(comma("f(,)), a(,(),)"), vec!["f(,))", "a(,(),)"]);
  }

  #[test]
  fn comma_keeps_escaped_commas() {
    assert_eq!(comma("a\\, b"), vec!["a\\, b"]);
  }

  #[test]
  fn split_allows_custom_separators() {
    assert_eq!(split("a|b|c", &['|'], false), vec!["a", "b", "c"]);
  }
}
