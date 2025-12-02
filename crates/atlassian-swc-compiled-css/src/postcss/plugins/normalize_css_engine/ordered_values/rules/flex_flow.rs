use crate::postcss::value_parser as vp;

// Port of src/rules/flexFlow.js
pub fn normalize(parsed: &vp::ParsedValue) -> String {
  let directions: std::collections::HashSet<&'static str> =
    ["row", "row-reverse", "column", "column-reverse"]
      .into_iter()
      .collect();
  let wraps: std::collections::HashSet<&'static str> =
    ["nowrap", "wrap", "wrap-reverse"].into_iter().collect();
  let mut direction = String::new();
  let mut wrap = String::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        let low = value.to_lowercase();
        if directions.contains(low.as_str()) {
          direction = value.clone();
        } else if wraps.contains(low.as_str()) {
          wrap = value.clone();
        }
      }
      true
    },
    false,
  );
  format!("{} {}", direction, wrap).trim().to_string()
}
