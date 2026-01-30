use super::super::library::{add_space::add_space, get_value::get_value};
use crate::postcss::value_parser as vp;

// Strict port of src/rules/border.js — assemble nodes with explicit spaces
pub fn normalize(parsed: &vp::ParsedValue) -> String {
  fn is_style(v: &str) -> bool {
    matches!(
      v,
      "none"
        | "auto"
        | "hidden"
        | "dotted"
        | "dashed"
        | "solid"
        | "double"
        | "groove"
        | "ridge"
        | "inset"
        | "outset"
    )
  }
  fn is_width(v: &str) -> bool {
    matches!(v, "thin" | "medium" | "thick") || vp::unit::unit(v).is_some()
  }

  let mut width: Vec<vp::Node> = Vec::new();
  let mut style: Vec<vp::Node> = Vec::new();
  let mut color: Vec<vp::Node> = Vec::new();

  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      match node {
        vp::Node::Space { .. } => {}
        vp::Node::Word { value } => {
          let low = value.to_lowercase();
          if is_style(&low) {
            if style.is_empty() {
              style.push(node.clone());
              style.push(add_space());
            }
            return false;
          }
          if is_width(&low) {
            width.push(node.clone());
            width.push(add_space());
            return false;
          }
          color.push(node.clone());
          color.push(add_space());
          return false;
        }
        vp::Node::Function { value, .. } => {
          let low = value.to_lowercase();
          if matches!(low.as_str(), "calc" | "min" | "max" | "clamp") {
            width.push(node.clone());
            width.push(add_space());
          } else {
            color.push(node.clone());
            color.push(add_space());
          }
          return false;
        }
        _ => {}
      }
      true
    },
    false,
  );

  // Flatten in canonical order: width style color
  let mut combined: Vec<vp::Node> = Vec::new();
  combined.extend(width);
  combined.extend(style);
  combined.extend(color);
  get_value(vec![combined])
}
