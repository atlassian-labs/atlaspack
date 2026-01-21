use once_cell::sync::Lazy;
use regex::Regex;

/// Regex mirroring the Babel helper that strips any occurrence of `{` or `}`
/// inside single or double quotes (and the literal `|` that the JS pattern
/// also matches) so brace pairs inside string literals do not interfere with
/// selector detection.
static QUOTED_BRACES: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"['|\"].*[{|}].*['|\"]"#).expect("invalid quoted brace regex"));

/// Regex that matches selectors which have not yet been closed. This mirrors
/// the `/[^;\s].+\n?{/g` pattern used in the Babel implementation.
static SELECTOR_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"[^;\s].+\n?\{").expect("invalid selector regex"));

/// Mirrors the Babel helper by returning selectors that are part of incomplete
/// closures. These selectors are used to reapply conditional CSS rules when
/// unconditional declarations introduce nested scopes.
pub fn find_open_selectors(css: &str) -> Option<Vec<String>> {
  let without_quoted = QUOTED_BRACES.replace_all(css, "");
  let search_area = match without_quoted.rfind('}') {
    Some(index) => &without_quoted[index + 1..],
    None => without_quoted.as_ref(),
  };

  let selectors: Vec<String> = SELECTOR_RE
    .find_iter(search_area)
    .map(|m| m.as_str().to_string())
    .collect();

  if selectors.is_empty() {
    None
  } else {
    Some(selectors)
  }
}

#[cfg(test)]
mod tests {
  use super::find_open_selectors;

  #[test]
  fn returns_none_when_no_open_selectors() {
    assert!(find_open_selectors(".a { color: red; }").is_none());
  }

  #[test]
  fn returns_open_selectors_from_nested_rules() {
    let css = ".a { color: red;\n  &:hover {";
    let selectors = find_open_selectors(css).expect("selectors");
    assert_eq!(selectors, vec![".a {", "&:hover {"]);
  }

  #[test]
  fn ignores_braces_inside_quotes() {
    let css = ".a { content: '{';\n  &:hover {";
    let selectors = find_open_selectors(css).expect("selectors");
    assert_eq!(selectors, vec![".a {", "&:hover {"]);
  }
}
