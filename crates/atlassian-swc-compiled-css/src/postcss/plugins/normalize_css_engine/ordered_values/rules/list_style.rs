use super::super::data::list_style_types::list_style_types;
use crate::postcss::value_parser as vp;

// Port of src/rules/listStyle.js
pub fn normalize(parsed: &vp::ParsedValue) -> String {
  let positions: std::collections::HashSet<&'static str> =
    ["inside", "outside"].into_iter().collect();
  let mut typ = String::new();
  let mut pos = String::new();
  let mut img = String::new();
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      match node {
        vp::Node::Word { value } => {
          let v = value.as_str();
          if list_style_types().contains(v) {
            typ.push(' ');
            typ.push_str(v);
          } else if positions.contains(v) {
            pos.push(' ');
            pos.push_str(v);
          } else if v == "none" {
            if typ.split(' ').any(|e| e == "none") {
              img.push(' ');
              img.push_str(v);
            } else {
              typ.push(' ');
              typ.push_str(v);
            }
          } else {
            typ.push(' ');
            typ.push_str(v);
          }
        }
        vp::Node::Function { .. } => {
          img.push(' ');
          img.push_str(&vp::stringify(&[node.clone()]));
        }
        _ => {}
      }
      true
    },
    false,
  );
  format!("{} {} {}", typ.trim(), pos.trim(), img.trim())
    .trim()
    .to_string()
}
