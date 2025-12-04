use crate::postcss::value_parser as vp;

// Port of src/rules/columns.js
pub fn normalize(parsed: &vp::ParsedValue) -> String {
  let mut widths: Vec<String> = Vec::new();
  let mut other: Vec<String> = Vec::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        if let Some(u) = vp::unit::unit(value) {
          if !u.unit.is_empty() {
            widths.push(value.clone());
          } else {
            other.push(value.clone());
          }
        } else {
          other.push(value.clone());
        }
      }
      true
    },
    false,
  );
  if other.len() == 1 && widths.len() == 1 {
    return format!("{} {}", widths[0].trim_start(), other[0].trim_start());
  }
  vp::stringify(&parsed.nodes)
}
