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
