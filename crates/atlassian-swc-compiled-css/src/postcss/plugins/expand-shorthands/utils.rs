use super::types::ValueNode;

pub const GLOBAL_VALUES: &[&str] = &["inherit", "initial", "unset", "revert", "revert-layer"];

pub fn is_color(node: &ValueNode) -> bool {
  node.is_color()
}

pub fn is_width(node: &ValueNode) -> bool {
  node.is_width()
}

pub fn get_width(node: &ValueNode) -> Option<String> {
  node.width_string()
}
