use crate::postcss::transform::{Plugin, TransformContext};
use swc_core::css::ast::Stylesheet;

/// Native translation of `postcss-discard-comments` that operates on the raw
/// CSS source prior to parsing. The SWC parser drops comment nodes, so the
/// plugin's primary responsibility in Rust is to capture the comments that the
/// PostCSS stack would have preserved (notably `/*! … */` license banners when
/// optimisations are enabled, or _all_ comments when optimisations are
/// disabled) so they can be re-emitted after transformation.
#[derive(Debug, Default, Clone, Copy)]
pub struct DiscardComments;

impl Plugin for DiscardComments {
  fn name(&self) -> &'static str {
    "postcss-discard-comments"
  }

  fn run(&self, _stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    // No-op. Comment capture occurs before parsing; we simply mirror the
    // plugin boundary to preserve the original pipeline structure.
  }
}

pub fn discard_comments() -> DiscardComments {
  DiscardComments
}

/// Collect the comments that should be preserved according to
/// `postcss-discard-comments`' default semantics.
///
/// When `optimize_css` is enabled we keep only "important" comments – those
/// whose contents begin with `!`. When optimisations are disabled we retain all
/// comments so the non-minified output mirrors Babel's behaviour.
pub fn collect_preserved_comments(css: &str, optimize_css: Option<bool>) -> Vec<String> {
  let mut preserved = Vec::new();
  let keep_only_important = optimize_css.unwrap_or(true);

  let bytes = css.as_bytes();
  let mut index = 0;

  while index + 1 < bytes.len() {
    if bytes[index] == b'/' && bytes[index + 1] == b'*' {
      if let Some(end) = find_comment_end(bytes, index + 2) {
        let body = &css[index + 2..end];
        let is_important = body.starts_with('!');

        if !keep_only_important || is_important {
          let mut comment = format!("/*{}*/", body);
          let mut cursor = end + 2;
          while cursor < bytes.len() {
            let ch = bytes[cursor];
            if matches!(ch, b' ' | b'\t' | b'\n' | b'\r') {
              comment.push(ch as char);
              cursor += 1;
            } else {
              break;
            }
          }
          preserved.push(comment);
          index = cursor;
          continue;
        }

        index = end + 2;
        continue;
      } else {
        // Unterminated comment – conservatively capture the rest.
        let body = &css[index + 2..];
        if !keep_only_important || body.starts_with('!') {
          preserved.push(format!("/*{}", body));
        }
        break;
      }
    }

    index += 1;
  }

  preserved
}

fn find_comment_end(bytes: &[u8], mut index: usize) -> Option<usize> {
  while index + 1 < bytes.len() {
    if bytes[index] == b'*' && bytes[index + 1] == b'/' {
      return Some(index);
    }
    index += 1;
  }
  None
}

#[cfg(test)]
mod tests {
  use super::collect_preserved_comments;

  #[test]
  fn collects_important_comments_when_optimising() {
    let css = "/*! keep */ .a { color: red; } /* drop */";
    let preserved = collect_preserved_comments(css, Some(true));
    assert_eq!(preserved, vec!["/*! keep */ ".to_string()]);
  }

  #[test]
  fn collects_all_comments_when_not_optimising() {
    let css = "/* first */ .a { /* second */ color: red; }";
    let preserved = collect_preserved_comments(css, Some(false));
    assert_eq!(
      preserved,
      vec!["/* first */ ".to_string(), "/* second */ ".to_string()]
    );
  }
}
