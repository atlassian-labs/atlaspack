use crate::postcss::plugins::normalize_css_engine::ordered_values::lib::arguments::get_arguments;
use crate::postcss::value_parser as vp;
use postcss as pc;
use regex::Regex;
use std::str::FromStr;

fn split_args_indices(nodes: &Vec<vp::Node>) -> Vec<Vec<usize>> {
  let mut args: Vec<Vec<usize>> = vec![Vec::new()];
  let mut depth = 0i32;
  for (i, n) in nodes.iter().enumerate() {
    match n {
      vp::Node::Function { .. } => {
        args.last_mut().unwrap().push(i);
        depth += 1;
      }
      vp::Node::Div { value, .. } if depth == 0 && value == "," => {
        args.push(Vec::new());
      }
      _ => {
        args.last_mut().unwrap().push(i);
        if matches!(n, vp::Node::Function { .. }) {
          depth -= 1;
        }
      }
    }
  }
  args
}

fn unit_of(value: &str) -> Option<(String, String)> {
  vp::unit::unit(value).map(|u| (u.number, u.unit.to_lowercase()))
}

fn is_css_length_unit(unit: &str) -> bool {
  matches!(
    unit.to_uppercase().as_str(),
    "PX"
      | "IN"
      | "CM"
      | "MM"
      | "EM"
      | "REM"
      | "POINTS"
      | "PC"
      | "EX"
      | "CH"
      | "VW"
      | "VH"
      | "VMIN"
      | "VMAX"
      | "%"
  )
}

fn is_stop(stop: Option<&str>) -> bool {
  if let Some(s) = stop {
    if let Some((num, uni)) = unit_of(s) {
      return num == "0" || is_css_length_unit(&uni);
    }
    return Regex::new(r"^calc\(\S+\)$").unwrap().is_match(s);
  }
  true
}

fn is_color_stop(color: &str, stop: Option<&str>) -> bool {
  csscolorparser::Color::from_str(color).is_ok() && is_stop(stop)
}

pub fn plugin() -> pc::BuiltPlugin {
  pc::plugin("postcss-minify-gradients")
        .decl(|decl, _| {
            let value = decl.value(); if value.is_empty() { return Ok(()); }
            let normalized = value.to_ascii_lowercase();
            if normalized.contains("var(") || normalized.contains("env(") { return Ok(()); }
            if !normalized.contains("gradient") { return Ok(()); }

            let mut parsed = vp::parse(&value);
            let mut changed = false;
            vp::walk(&mut parsed.nodes[..], &mut |n| {
                if let vp::Node::Function { value: fname, nodes, .. } = n {
                    let name = fname.to_ascii_lowercase();
                    if matches!(name.as_str(), "linear-gradient"|"repeating-linear-gradient"|"-webkit-linear-gradient"|"-webkit-repeating-linear-gradient") {
                        let mut args = get_arguments(&vp::ParsedValue { nodes: nodes.clone() });
                        if let Some(first_tokens) = args.get_mut(0) {
                            if !first_tokens.is_empty() {
                                if let vp::Node::Word { value: ref mut first } = first_tokens[0] {
                                    if first.to_ascii_lowercase() == "to" && first_tokens.len() == 3 {
                                        // slice(2)
                                        nodes.drain(0..2);
                                        if let Some(vp::Node::Word { value: side }) = nodes.get_mut(0) {
                                            let lv = side.to_ascii_lowercase();
                                            let mapped = match lv.as_str() {
                                                "top" => "0deg".to_string(),
                                                "right" => "90deg".to_string(),
                                                "bottom" => "180deg".to_string(),
                                                "left" => "270deg".to_string(),
                                                _ => lv.clone(),
                                            };
                                            *side = mapped;
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                        // process color stops
                        // angle keyword conversion
                        if nodes.len() >= 3 {
                            if let vp::Node::Word { value: first } = &nodes[0] {
                                if first.eq_ignore_ascii_case("to") {
                                    let side = if let vp::Node::Word { value: s } = &nodes[2] { s.to_ascii_lowercase() } else { String::new() };
                                    nodes.drain(0..2);
                                    if let Some(vp::Node::Word { value: v0 }) = nodes.get_mut(0) {
                                        let mapped = match side.as_str() {
                                            "top" => "0deg".to_string(),
                                            "right" => "90deg".to_string(),
                                            "bottom" => "180deg".to_string(),
                                            "left" => "270deg".to_string(),
                                            _ => side.clone(),
                                        };
                                        *v0 = mapped;
                                        changed = true;
                                    }
                                }
                            }
                        }
                        let arg_indices = split_args_indices(nodes);
                        let mut last_stop: Option<(String,String)> = None;
                        for (idx, arg_idx) in arg_indices.iter().enumerate() {
                            if arg_idx.len() < 3 { continue; }
                            let is_final = idx == arg_indices.len() - 1;
                            let stop_i = arg_idx[arg_idx.len()-1];
                            let mid_i = arg_idx[arg_idx.len()-2];
                            let stop_val_s = match &nodes[stop_i] { vp::Node::Word { value } => Some(value.clone()), vp::Node::Function { value, nodes: inner, .. } => Some(format!("{}({})", value, vp::stringify(inner))), _ => None };
                            let this_stop = stop_val_s.as_deref().and_then(|s| unit_of(s));
                            if last_stop.is_none() {
                                last_stop = this_stop.clone();
                                if !is_final { if let Some((ref num, ref unit)) = last_stop { if num == "0" && unit.to_ascii_lowercase() != "deg" {
                                    if let vp::Node::Word { value } = &mut nodes[mid_i] { *value = String::new(); }
                                    if let vp::Node::Word { value } = &mut nodes[stop_i] { *value = String::new(); }
                                    changed = true; } } }
                                continue;
                            }
                            if let (Some((ln, lu)), Some((rn, ru))) = (last_stop.clone(), this_stop.clone()) {
                                if lu.eq_ignore_ascii_case(&ru) && ln.parse::<f32>().ok().unwrap_or(0.0) >= rn.parse::<f32>().ok().unwrap_or(0.0) {
                                    if let vp::Node::Word { value } = &mut nodes[stop_i] { *value = "0".to_string(); changed = true; }
                                }
                            }
                            last_stop = this_stop;
                            if is_final {
                                if mid_i != stop_i {
                                    let (i1, i2) = if mid_i < stop_i { (mid_i, stop_i) } else { (stop_i, mid_i) };
                                    let (left, right) = nodes.split_at_mut(i2);
                                    let left_node = &mut left[i1];
                                    let right_node = &mut right[0];
                                    // Map back which is which
                                    let (mid_node, stop_node) = if mid_i < stop_i { (left_node, right_node) } else { (right_node, left_node) };
                                    if let vp::Node::Word { value: stop_val } = stop_node {
                                        if stop_val == "100%" {
                                            if let vp::Node::Word { value: mid_val } = mid_node { *mid_val = String::new(); }
                                            *stop_val = String::new();
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    } else if matches!(name.as_str(), "radial-gradient"|"repeating-radial-gradient") {
                        let arg_indices = split_args_indices(nodes);
                        let mut last_stop: Option<(String,String)> = None;
                        let has_at = arg_indices.get(0).map(|v| v.iter().any(|i| matches!(&nodes[*i], vp::Node::Word{value} if value.eq_ignore_ascii_case("at")))).unwrap_or(false);
                        for (idx,arg_idx) in arg_indices.iter().enumerate() {
                            if arg_idx.len() < 3 || (idx == 0 && has_at) { continue; }
                            let stop_i = arg_idx[arg_idx.len()-1];
                            let stop_val_s = match &nodes[stop_i] { vp::Node::Word { value } => Some(value.clone()), vp::Node::Function { value, nodes: inner, .. } => Some(format!("{}({})", value, vp::stringify(inner))), _ => None };
                            let this_stop = stop_val_s.as_deref().and_then(|s| unit_of(s));
                            if last_stop.is_none() { last_stop = this_stop; continue; }
                            if let (Some((ln, lu)), Some((rn, ru))) = (last_stop.clone(), this_stop.clone()) {
                                if lu.eq_ignore_ascii_case(&ru) && ln.parse::<f32>().ok().unwrap_or(0.0) >= rn.parse::<f32>().ok().unwrap_or(0.0) {
                                    if let vp::Node::Word { value } = &mut nodes[stop_i] { *value = "0".to_string(); changed = true; }
                                }
                            }
                            last_stop = this_stop;
                        }
                    } else if matches!(name.as_str(), "-webkit-radial-gradient"|"-webkit-repeating-radial-gradient") {
                        let arg_indices = split_args_indices(nodes);
                        let mut last_stop: Option<(String,String)> = None;
                        for arg_idx in arg_indices.iter() {
                            let mut color: Option<String> = None; let mut stop: Option<String> = None;
                            if arg_idx.len() >= 3 {
                                let c0 = arg_idx[0];
                                if let vp::Node::Function{value, nodes: inner, ..} = &nodes[c0] { color = Some(format!("{}({})", value, vp::stringify(inner))); }
                                else if let vp::Node::Word{value} = &nodes[c0] { color = Some(value.clone()); }
                                let s2 = arg_idx[arg_idx.len()-1];
                                if let vp::Node::Function{value, nodes: inner, ..} = &nodes[s2] { stop = Some(format!("{}({})", value, vp::stringify(inner))); }
                                else if let vp::Node::Word{value} = &nodes[s2] { stop = Some(value.clone()); }
                            } else if !arg_idx.is_empty() {
                                let c0 = arg_idx[0];
                                if let vp::Node::Function{value, nodes: inner, ..} = &nodes[c0] { color = Some(format!("{}({})", value, vp::stringify(inner))); }
                                else if let vp::Node::Word{value} = &nodes[c0] { color = Some(value.clone()); }
                            }
                            let color_lc = color.unwrap_or_default().to_ascii_lowercase();
                            let color_stop = if let Some(ref s) = stop { is_color_stop(&color_lc, Some(&s.to_ascii_lowercase())) } else { is_color_stop(&color_lc, None) };
                            if !color_stop || arg_idx.len() < 3 { continue; }
                            let s2 = arg_idx[arg_idx.len()-1];
                            let this_stop = stop.as_deref().and_then(|s| unit_of(s));
                            if last_stop.is_none() { last_stop = this_stop; continue; }
                            if let (Some((ln, lu)), Some((rn, ru))) = (last_stop.clone(), this_stop.clone()) {
                                if lu.eq_ignore_ascii_case(&ru) && ln.parse::<f32>().ok().unwrap_or(0.0) >= rn.parse::<f32>().ok().unwrap_or(0.0) {
                                    if let vp::Node::Word { value } = &mut nodes[s2] { *value = "0".to_string(); changed = true; }
                                }
                            }
                            last_stop = this_stop;
                        }
                    }
                }
                true
            }, false);
            if changed { decl.set_value(vp::stringify(&parsed.nodes)); }
            Ok(())
        })
        .build()
}
