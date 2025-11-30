use super::super::lib::{
  add_space::add_space, arguments::get_arguments, get_value::get_value, mathfunctions,
  vendor_unprefixed::vendor_unprefixed,
};
use crate::postcss::value_parser as vp;

// Port of src/rules/boxShadow.js
pub fn normalize(parsed: &vp::ParsedValue) -> Result<String, ()> {
  let args = get_arguments(parsed);
  let mut list: Vec<Vec<vp::Node>> = Vec::new();
  for arg in args.into_iter() {
    let mut val: Vec<vp::Node> = Vec::new();
    let mut inset: Vec<vp::Node> = Vec::new();
    let mut color: Vec<vp::Node> = Vec::new();
    let mut abort = false;
    for node in arg.into_iter() {
      match &node {
        vp::Node::Function { value, .. } => {
          let low = vendor_unprefixed(value.to_lowercase().as_str()).to_string();
          if mathfunctions::set().contains(low.as_str()) {
            abort = true;
            break;
          }
          color.push(node.clone());
          color.push(add_space());
        }
        vp::Node::Space { .. } => {}
        vp::Node::Word { value } => {
          if vp::unit::unit(value).is_some() {
            val.push(node.clone());
            val.push(add_space());
          } else if value.to_lowercase() == "inset" {
            inset.push(node.clone());
            inset.push(add_space());
          } else {
            color.push(node.clone());
            color.push(add_space());
          }
        }
        _ => {}
      }
    }
    if abort {
      return Err(());
    }
    let mut combined: Vec<vp::Node> = Vec::new();
    combined.extend(inset);
    combined.extend(val);
    combined.extend(color);
    list.push(combined);
  }
  Ok(get_value(list))
}
