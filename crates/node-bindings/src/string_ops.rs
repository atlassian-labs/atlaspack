use aho_corasick::{AhoCorasick, MatchKind};
use napi_derive::napi;

#[napi(object)]
pub struct Replacement {
  pub from: String,
  pub to: String,
}

#[napi]
pub fn perform_string_replacements(input: String, replacements: Vec<Replacement>) -> String {
  let mut froms = Vec::new();
  let mut tos = Vec::new();

  for replacement in &replacements {
    froms.push(replacement.from.as_str());
    tos.push(replacement.to.as_str());
  }

  let ac = AhoCorasick::builder()
    .match_kind(MatchKind::Standard)
    .build(froms)
    .unwrap();

  ac.replace_all(&input, &tos)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_replacement() {
    let input = String::from("Hello world");
    let replacements = vec![Replacement {
      from: String::from("world"),
      to: String::from("Rust"),
    }];
    let result = perform_string_replacements(input, replacements);
    assert_eq!(result, "Hello Rust");
  }

  #[test]
  fn test_multiple_replacements() {
    let input = String::from("The quick brown fox jumps over the lazy dog");
    let replacements = vec![
      Replacement {
        from: String::from("quick"),
        to: String::from("slow"),
      },
      Replacement {
        from: String::from("brown"),
        to: String::from("red"),
      },
      Replacement {
        from: String::from("lazy"),
        to: String::from("energetic"),
      },
    ];
    let result = perform_string_replacements(input, replacements);
    assert_eq!(result, "The slow red fox jumps over the energetic dog");
  }

  #[test]
  fn test_overlapping_patterns() {
    let input = String::from("aaaa");
    let replacements = vec![Replacement {
      from: String::from("aa"),
      to: String::from("b"),
    }];
    let result = perform_string_replacements(input, replacements);
    assert_eq!(result, "bb");
  }

  #[test]
  fn test_empty_input() {
    let input = String::from("");
    let replacements = vec![Replacement {
      from: String::from("test"),
      to: String::from("replacement"),
    }];
    let result = perform_string_replacements(input, replacements);
    assert_eq!(result, "");
  }

  #[test]
  fn test_no_replacements() {
    let input = String::from("Hello world");
    let replacements = vec![];
    let result = perform_string_replacements(input, replacements);
    assert_eq!(result, "Hello world");
  }

  #[test]
  fn test_replacement_with_empty_string() {
    let input = String::from("Remove this text");
    let replacements = vec![Replacement {
      from: String::from("this text"),
      to: String::from(""),
    }];
    let result = perform_string_replacements(input, replacements);
    assert_eq!(result, "Remove ");
  }
}
