use super::super::lib::{add_space::add_space, arguments::get_arguments, get_value::get_value};
use crate::postcss::value_parser as vp;

// Port (subset) of src/rules/animation.js
pub fn normalize(parsed: &vp::ParsedValue) -> String {
  let args = get_arguments(parsed);
  let mut out_lists: Vec<Vec<vp::Node>> = Vec::new();
  for arg in args.into_iter() {
    let mut name: Vec<vp::Node> = Vec::new();
    let mut duration: Vec<vp::Node> = Vec::new();
    let mut timing: Vec<vp::Node> = Vec::new();
    let mut delay: Vec<vp::Node> = Vec::new();
    let mut iteration: Vec<vp::Node> = Vec::new();
    let mut direction: Vec<vp::Node> = Vec::new();
    let mut fill: Vec<vp::Node> = Vec::new();
    let mut play: Vec<vp::Node> = Vec::new();
    for node in arg.into_iter() {
      match &node {
        vp::Node::Space { .. } => {}
        vp::Node::Function { value, .. } => {
          let low = value.to_lowercase();
          if low == "steps" || low == "cubic-bezier" || low == "frames" {
            if timing.is_empty() {
              timing.push(node.clone());
              timing.push(add_space());
            } else {
              name.push(node.clone());
              name.push(add_space());
            }
          } else {
            name.push(node.clone());
            name.push(add_space());
          }
        }
        vp::Node::Word { value } => {
          let low = value.to_lowercase();
          if let Some(u) = vp::unit::unit(value) {
            if u.unit == "ms" || u.unit == "s" {
              if duration.is_empty() {
                duration.push(node.clone());
                duration.push(add_space());
              } else if delay.is_empty() {
                delay.push(node.clone());
                delay.push(add_space());
              } else {
                name.push(node.clone());
                name.push(add_space());
              }
              continue;
            }
            if u.unit.is_empty() {
              if iteration.is_empty() {
                iteration.push(node.clone());
                iteration.push(add_space());
                continue;
              }
            }
          }
          if matches!(
            low.as_str(),
            "ease" | "ease-in" | "ease-in-out" | "ease-out" | "linear" | "step-end" | "step-start"
          ) {
            if timing.is_empty() {
              timing.push(node.clone());
              timing.push(add_space());
            } else {
              name.push(node.clone());
              name.push(add_space());
            }
          } else if matches!(
            low.as_str(),
            "normal" | "reverse" | "alternate" | "alternate-reverse"
          ) {
            if direction.is_empty() {
              direction.push(node.clone());
              direction.push(add_space());
            }
          } else if matches!(low.as_str(), "none" | "forwards" | "backwards" | "both") {
            if fill.is_empty() {
              fill.push(node.clone());
              fill.push(add_space());
            }
          } else if matches!(low.as_str(), "running" | "paused") {
            if play.is_empty() {
              play.push(node.clone());
              play.push(add_space());
            }
          } else if low.as_str() == "infinite" {
            if iteration.is_empty() {
              iteration.push(node.clone());
              iteration.push(add_space());
            }
          } else {
            name.push(node.clone());
            name.push(add_space());
          }
        }
        _ => {
          name.push(node.clone());
          name.push(add_space());
        }
      }
    }
    let mut combined: Vec<vp::Node> = Vec::new();
    combined.extend(name);
    combined.extend(duration);
    combined.extend(timing);
    combined.extend(delay);
    combined.extend(iteration);
    combined.extend(direction);
    combined.extend(fill);
    combined.extend(play);
    out_lists.push(combined);
  }
  get_value(out_lists)
}
