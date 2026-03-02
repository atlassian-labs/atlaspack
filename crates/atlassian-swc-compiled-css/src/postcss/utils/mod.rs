pub mod selector_stringifier;
pub mod sort_pseudo_selectors;
pub mod style_ordering;
pub mod value_minifier;

pub(crate) fn collapse_adjacent_nesting_selectors(selector: &str) -> String {
  let mut out = String::with_capacity(selector.len());
  let mut chars = selector.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '&' {
      out.push('&');
      let mut saw_ws = false;
      while let Some(next) = chars.peek() {
        if next.is_whitespace() {
          saw_ws = true;
          chars.next();
        } else {
          break;
        }
      }
      if let Some('&') = chars.peek().copied() {
        continue;
      }
      if saw_ws {
        out.push(' ');
      }
      continue;
    }
    out.push(ch);
  }
  out
}
