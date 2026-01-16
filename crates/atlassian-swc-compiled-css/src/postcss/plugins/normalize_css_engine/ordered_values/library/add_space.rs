use crate::postcss::value_parser as vp;

pub fn add_space() -> vp::Node {
  vp::Node::Space {
    value: " ".to_string(),
  }
}
