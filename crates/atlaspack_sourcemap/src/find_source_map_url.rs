use std::sync::LazyLock;

use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct SourceMapUrlMatch {
  /// The code containing the sourcemap url
  pub code: String,

  /// The url to the sourcemap
  pub url: String,
}

static SOURCEMAP_URL: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"\/[*/][@#]\s*sourceMappingURL\s*=\s*(?<url>\S+)(?:\s*\*\/)?\s*$").unwrap()
});

pub fn find_sourcemap_url(code: &str) -> Option<SourceMapUrlMatch> {
  if let Some(captures) = SOURCEMAP_URL.captures(code) {
    if let (Some(code), Some(url)) = (captures.get(0), captures.name("url")) {
      return Some(SourceMapUrlMatch {
        code: code.as_str().trim().into(),
        url: url.as_str().to_string(),
      });
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn returns_none_when_no_sourcemap_urls() {
    assert_eq!(find_sourcemap_url(""), None);
    assert_eq!(
      find_sourcemap_url("/// sourceMappingURL=index.js.map"),
      None
    );
  }

  #[test]
  fn returns_deprecated_sourcemap_url() {
    assert_eq!(
      find_sourcemap_url(
        r"
          console.log('test');

          //@ sourceMappingURL=index.js.map
        "
      ),
      Some(SourceMapUrlMatch {
        code: String::from("//@ sourceMappingURL=index.js.map"),
        url: String::from("index.js.map")
      })
    );
  }

  #[test]
  fn returns_sourcemap_url() {
    assert_eq!(
      find_sourcemap_url(
        r"
          console.log('test');

          //# sourceMappingURL=index.js.map
        "
      ),
      Some(SourceMapUrlMatch {
        code: String::from("//# sourceMappingURL=index.js.map"),
        url: String::from("index.js.map")
      })
    );
  }

  #[test]
  fn returns_css_sourcemap_url() {
    assert_eq!(
      find_sourcemap_url(
        r"
          a {
            color: blue;
          }

          /*# sourceMappingURL=index.css.map */
        "
      ),
      Some(SourceMapUrlMatch {
        code: String::from("/*# sourceMappingURL=index.css.map */"),
        url: String::from("index.css.map")
      })
    );
  }
}
