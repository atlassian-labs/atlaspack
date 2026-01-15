use super::super::library::{add_space::add_space, arguments::get_arguments, get_value::get_value};
use crate::postcss::value_parser as vp;

// Port of src/rules/transition.js
pub fn normalize(parsed: &vp::ParsedValue) -> String {
  let args = get_arguments(parsed);
  let lists = normalize_args(args);
  get_value(lists)
}

fn normalize_args(args: Vec<Vec<vp::Node>>) -> Vec<Vec<vp::Node>> {
  let mut list: Vec<Vec<vp::Node>> = Vec::new();
  for arg in args.into_iter() {
    let mut timing: Vec<vp::Node> = Vec::new();
    let mut property: Vec<vp::Node> = Vec::new();
    let mut time1: Vec<vp::Node> = Vec::new();
    let mut time2: Vec<vp::Node> = Vec::new();
    for node in arg.into_iter() {
      match &node {
        vp::Node::Space { .. } => {}
        vp::Node::Function { value, .. } => {
          let low = value.to_lowercase();
          if low == "steps" || low == "cubic-bezier" {
            timing.push(node.clone());
            timing.push(add_space());
          } else {
            property.push(node.clone());
            property.push(add_space());
          }
        }
        vp::Node::Word { value } => {
          if vp::unit::unit(value).is_some() {
            if time1.is_empty() {
              time1.push(node.clone());
              time1.push(add_space());
            } else {
              time2.push(node.clone());
              time2.push(add_space());
            }
          } else {
            let low = value.to_lowercase();
            if matches!(
              low.as_str(),
              "ease"
                | "linear"
                | "ease-in"
                | "ease-out"
                | "ease-in-out"
                | "step-start"
                | "step-end"
            ) {
              timing.push(node.clone());
              timing.push(add_space());
            } else {
              property.push(node.clone());
              property.push(add_space());
            }
          }
        }
        _ => {
          property.push(node.clone());
          property.push(add_space());
        }
      }
    }
    let mut combined: Vec<vp::Node> = Vec::new();
    combined.extend(property);
    combined.extend(time1);
    combined.extend(timing);
    combined.extend(time2);
    list.push(combined);
  }
  list
}
