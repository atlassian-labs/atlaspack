use crate::postcss::plugins::normalize_css_engine::ordered_values::library::arguments::get_arguments;
use crate::postcss::value_parser as vp;
use once_cell::sync::Lazy;
use postcss as pc;
use regex::Regex;
use std::str::FromStr;

static CALC_VALUE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^calc\(\S+\)$").unwrap());

/// Transform a CSS value for hash computation, applying gradient minification.
/// This mirrors the minify_gradients PostCSS plugin behavior for hash consistency.
/// IMPORTANT: This function preserves whitespace (newlines, tabs) because Babel's
/// cssnano runs BEFORE atomicify, and whitespace normalization runs AFTER atomicify.
/// The hash must be computed from the value with preserved whitespace.
pub fn transform_value_for_hash(value: &str) -> String {
  let normalized = value.to_ascii_lowercase();
  if !normalized.contains("gradient") {
    return value.to_string();
  }
  // Skip values with var() or env() as we don't want to transform those
  if normalized.contains("var(") || normalized.contains("env(") {
    return value.to_string();
  }

  let mut parsed = vp::parse(value);
  let mut changed = false;

  vp::walk(
    &mut parsed.nodes[..],
    &mut |n| {
      if let vp::Node::Function {
        value: fname,
        nodes,
        ..
      } = n
      {
        let name = fname.to_ascii_lowercase();
        if matches!(
          name.as_str(),
          "linear-gradient"
            | "repeating-linear-gradient"
            | "-webkit-linear-gradient"
            | "-webkit-repeating-linear-gradient"
        ) {
          let arg_indices = split_args_indices(nodes);

          // Only process final 100% removal - preserve all other whitespace
          if let Some(last_arg_idx) = arg_indices.last() {
            if last_arg_idx.len() >= 2 {
              // Find the last Word node (the stop value)
              let stop_i = last_arg_idx
                .iter()
                .rev()
                .copied()
                .find(|&i| matches!(&nodes[i], vp::Node::Word { .. }));

              if let Some(stop_i) = stop_i {
                if let vp::Node::Word { value: stop_val } = &nodes[stop_i] {
                  if stop_val == "100%" {
                    // Find the space before the 100%
                    let mid_i = last_arg_idx
                      .iter()
                      .position(|&i| i == stop_i)
                      .and_then(|pos| {
                        if pos > 0 {
                          Some(last_arg_idx[pos - 1])
                        } else {
                          None
                        }
                      });

                    // Clear the space before 100%
                    if let Some(mid_i) = mid_i {
                      if let vp::Node::Space { value } = &mut nodes[mid_i] {
                        *value = String::new();
                      }
                    }
                    // Clear the 100%
                    if let vp::Node::Word { value } = &mut nodes[stop_i] {
                      *value = String::new();
                    }
                    changed = true;
                  }
                }
              }
            }
          }
        }
      }
      true
    },
    false,
  );

  if changed {
    vp::stringify(&parsed.nodes)
  } else {
    value.to_string()
  }
}

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
    return CALC_VALUE_REGEX.is_match(s);
  }
  true
}

fn is_color_stop(color: &str, stop: Option<&str>) -> bool {
  csscolorparser::Color::from_str(color).is_ok() && is_stop(stop)
}

fn normalize_gradient_spacing(nodes: &mut Vec<vp::Node>) -> bool {
  let mut changed = false;

  // Drop empty word/space nodes left behind by removals.
  let mut i = 0usize;
  while i < nodes.len() {
    let remove = match &nodes[i] {
      vp::Node::Word { value } => value.is_empty(),
      vp::Node::Space { value } => value.is_empty(),
      _ => false,
    };
    if remove {
      nodes.remove(i);
      changed = true;
    } else {
      i += 1;
    }
  }

  // Trim leading/trailing spaces.
  while matches!(nodes.first(), Some(vp::Node::Space { .. })) {
    nodes.remove(0);
    changed = true;
  }
  while matches!(nodes.last(), Some(vp::Node::Space { .. })) {
    nodes.pop();
    changed = true;
  }

  // Remove spaces before commas and collapse spaces after commas.
  let mut idx = 0usize;
  while idx < nodes.len() {
    let is_comma = matches!(
      nodes.get(idx),
      Some(vp::Node::Div { value, .. }) if value == ","
    );
    if is_comma {
      while idx > 0 {
        if matches!(
          nodes.get(idx.saturating_sub(1)),
          Some(vp::Node::Space { .. })
        ) {
          nodes.remove(idx - 1);
          idx = idx.saturating_sub(1);
          changed = true;
        } else {
          break;
        }
      }

      // Normalize spaces after commas: collapse multiple spaces to a single space
      // but PRESERVE ONE space (cssnano postcss-minify-gradients behavior)
      let had_space_after = matches!(nodes.get(idx + 1), Some(vp::Node::Space { .. }));
      while idx + 1 < nodes.len() {
        if matches!(nodes.get(idx + 1), Some(vp::Node::Space { .. })) {
          nodes.remove(idx + 1);
          changed = true;
        } else {
          break;
        }
      }
      // Re-insert a single space if there was one originally
      if had_space_after && idx + 1 < nodes.len() {
        nodes.insert(
          idx + 1,
          vp::Node::Space {
            value: " ".to_string(),
          },
        );
        idx += 1;
      }
    }
    idx += 1;
  }

  changed
}

/// Count leading space nodes in a Vec of nodes.
fn count_leading_spaces(nodes: &[vp::Node]) -> usize {
  nodes
    .iter()
    .take_while(|n| matches!(n, vp::Node::Space { .. }))
    .count()
}

/// Map gradient direction keyword to degrees.
fn direction_to_degrees(direction: &str) -> Option<&'static str> {
  match direction.to_ascii_lowercase().as_str() {
    "top" => Some("0deg"),
    "right" => Some("90deg"),
    "bottom" => Some("180deg"),
    "left" => Some("270deg"),
    _ => None,
  }
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
                        // Handle "to <direction>" syntax and convert to degrees.
                        // The nodes may have leading whitespace that we need to account for.
                        let leading_spaces = count_leading_spaces(nodes);
                        let args = get_arguments(&vp::ParsedValue { nodes: nodes.clone() });
                        if let Some(first_tokens) = args.first() {
                            // first_tokens has leading spaces stripped by get_arguments,
                            // so we check for [Word("to"), Space, Word(direction)]
                            if first_tokens.len() == 3 {
                                if let vp::Node::Word { value: first } = &first_tokens[0] {
                                    if first.eq_ignore_ascii_case("to") {
                                        if let vp::Node::Word { value: direction } = &first_tokens[2] {
                                            if let Some(degrees) = direction_to_degrees(direction) {
                                                // Drain: leading_spaces + "to" + space = leading_spaces + 2
                                                // Then set the first Word node (direction) to degrees
                                                let drain_count = leading_spaces + 2;
                                                if nodes.len() > drain_count {
                                                    nodes.drain(0..drain_count);
                                                    // Skip any remaining leading space after drain
                                                    while matches!(nodes.first(), Some(vp::Node::Space { .. })) {
                                                        nodes.remove(0);
                                                    }
                                                    // Now nodes[0] should be the direction keyword
                                                    if let Some(vp::Node::Word { value: side }) = nodes.get_mut(0) {
                                                        *side = degrees.to_string();
                                                        changed = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        let arg_indices = split_args_indices(nodes);
                        let mut last_stop: Option<(String,String)> = None;
                        // Track indices to clear for final 100% removal
                        let mut final_100_removal: Option<(usize, usize)> = None;

                        for (idx, arg_idx) in arg_indices.iter().enumerate() {
                            if arg_idx.len() < 3 { continue; }
                            let is_final = idx == arg_indices.len() - 1;

                            // Find the last Word or Function node (skipping trailing spaces)
                            let stop_i = arg_idx.iter().rev().copied().find(|&i| {
                                matches!(&nodes[i], vp::Node::Word { .. } | vp::Node::Function { .. })
                            }).unwrap_or(arg_idx[arg_idx.len()-1]);

                            // Find the space before stop_i (for mid position)
                            let mid_i = arg_idx.iter().position(|&i| i == stop_i)
                                .and_then(|pos| if pos > 0 { Some(arg_idx[pos - 1]) } else { None })
                                .unwrap_or(arg_idx[arg_idx.len().saturating_sub(2)]);

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
                                // Check if the stop value is "100%" and mark for removal
                                if let vp::Node::Word { value: stop_val } = &nodes[stop_i] {
                                    if stop_val == "100%" {
                                        final_100_removal = Some((mid_i, stop_i));
                                    }
                                }
                            }
                        }

                        // Apply 100% removal in a separate pass to avoid borrow issues
                        if let Some((mid_i, stop_i)) = final_100_removal {
                            // Clear the space before 100%
                            if let vp::Node::Space { value } = &mut nodes[mid_i] { *value = String::new(); }
                            // Clear the 100% value
                            if let vp::Node::Word { value } = &mut nodes[stop_i] { *value = String::new(); }
                            changed = true;
                        }
                        if normalize_gradient_spacing(nodes) {
                            changed = true;
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
                        if normalize_gradient_spacing(nodes) {
                            changed = true;
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
                        if normalize_gradient_spacing(nodes) {
                            changed = true;
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

#[cfg(test)]
mod tests {
  use super::plugin;
  use postcss as pc;
  use postcss::ast::nodes::as_declaration;
  use pretty_assertions::assert_eq;

  #[test]
  fn trims_extra_gradient_spacing_after_stop_removal() {
    let css = ".a{background: linear-gradient(90deg, #4d8ced 0%, #cfe1fd 100%);}";
    let mut result = pc::Processor::new()
      .with_plugin(plugin())
      .process(css)
      .expect("process css");
    let output = result.to_css_string().expect("stringify css");
    let root = pc::parse(&output).expect("parse output");
    let mut value = None;

    root.walk_decls(|node, _| {
      if let Some(decl) = as_declaration(&node) {
        if decl.prop() == "background" {
          value = Some(decl.value());
          return false;
        }
      }
      true
    });

    assert_eq!(
      value.expect("background decl"),
      "linear-gradient(90deg, #4d8ced, #cfe1fd)"
    );
  }

  #[test]
  fn converts_to_bottom_to_180deg() {
    let css = ".a{background: linear-gradient(to bottom, #101214, #0e1624);}";
    let mut result = pc::Processor::new()
      .with_plugin(plugin())
      .process(css)
      .expect("process css");
    let output = result.to_css_string().expect("stringify css");
    let root = pc::parse(&output).expect("parse output");
    let mut value = None;

    root.walk_decls(|node, _| {
      if let Some(decl) = as_declaration(&node) {
        if decl.prop() == "background" {
          value = Some(decl.value());
          return false;
        }
      }
      true
    });

    assert_eq!(
      value.expect("background decl"),
      "linear-gradient(180deg, #101214, #0e1624)"
    );
  }

  #[test]
  fn converts_to_top_to_0deg() {
    let css = ".a{background-image: linear-gradient(to top, #101214, #0e1624);}";
    let mut result = pc::Processor::new()
      .with_plugin(plugin())
      .process(css)
      .expect("process css");
    let output = result.to_css_string().expect("stringify css");
    let root = pc::parse(&output).expect("parse output");
    let mut value = None;

    root.walk_decls(|node, _| {
      if let Some(decl) = as_declaration(&node) {
        if decl.prop() == "background-image" {
          value = Some(decl.value());
          return false;
        }
      }
      true
    });

    assert_eq!(
      value.expect("background-image decl"),
      "linear-gradient(0deg, #101214, #0e1624)"
    );
  }

  #[test]
  fn converts_to_bottom_with_extra_spacing() {
    // This test case has extra spacing like in the real source file
    let css =
      ".a{background: linear-gradient( to bottom, #101214, rgba(14, 22, 36, 0) ) no-repeat;}";
    let mut result = pc::Processor::new()
      .with_plugin(plugin())
      .process(css)
      .expect("process css");
    let output = result.to_css_string().expect("stringify css");
    let root = pc::parse(&output).expect("parse output");
    let mut value = None;

    root.walk_decls(|node, _| {
      if let Some(decl) = as_declaration(&node) {
        if decl.prop() == "background" {
          value = Some(decl.value());
          return false;
        }
      }
      true
    });

    // Should convert "to bottom" to "180deg"
    assert!(
      value.as_ref().expect("background decl").contains("180deg"),
      "Expected 180deg but got: {:?}",
      value
    );
  }

  #[test]
  fn removes_100_percent_from_final_color_stop_without_spacing() {
    // This is the actual failing case - no spaces after commas
    let css = ".a{background:linear-gradient(90deg,#0065ff,#0469ff 12%,#bf63f3 24%,#ffa900 48%,#bf63f3 64%,#0469ff 80%,#0065ff 100%);}";
    let mut result = pc::Processor::new()
      .with_plugin(plugin())
      .process(css)
      .expect("process css");
    let output = result.to_css_string().expect("stringify css");
    let root = pc::parse(&output).expect("parse output");
    let mut value = None;

    root.walk_decls(|node, _| {
      if let Some(decl) = as_declaration(&node) {
        if decl.prop() == "background" {
          value = Some(decl.value());
          return false;
        }
      }
      true
    });

    // The 100% should be removed from the final color stop
    // Expected: linear-gradient(90deg,#0065ff,#0469ff 12%,#bf63f3 24%,#ffa900 48%,#bf63f3 64%,#0469ff 80%,#0065ff)
    let result = value.expect("background decl");
    assert!(
      !result.contains("100%"),
      "Expected 100%% to be removed from final stop but got: {}",
      result
    );
    // Should end with just the color (no percentage)
    assert!(
      result.contains("#0065ff)") || result.ends_with("#0065ff"),
      "Expected gradient to end with #0065ff without percentage but got: {}",
      result
    );
  }

  #[test]
  fn removes_100_percent_with_trailing_whitespace() {
    // This is the exact format from the real build output - CSS with newlines and tabs
    let css = ".a{background:linear-gradient(90deg,\n\t\t\t#0065ff 0,\n\t\t\t#0469ff 12%,\n\t\t\t#bf63f3 24%,\n\t\t\t#ffa900 48%,\n\t\t\t#bf63f3 64%,\n\t\t\t#0469ff 80%,\n\t\t\t#0065ff 100%\n\t\t\t);}";
    let mut result = pc::Processor::new()
      .with_plugin(plugin())
      .process(css)
      .expect("process css");
    let output = result.to_css_string().expect("stringify css");
    let root = pc::parse(&output).expect("parse output");
    let mut value = None;

    root.walk_decls(|node, _| {
      if let Some(decl) = as_declaration(&node) {
        if decl.prop() == "background" {
          value = Some(decl.value());
          return false;
        }
      }
      true
    });

    let result = value.expect("background decl");
    assert!(
      !result.contains("100%"),
      "Expected 100%% to be removed from final stop but got: {}",
      result
    );
  }

  #[test]
  fn transform_value_for_hash_removes_100_percent() {
    let value = "linear-gradient(90deg,#0065ff,#0469ff 12%,#bf63f3 24%,#ffa900 48%,#bf63f3 64%,#0469ff 80%,#0065ff 100%)";
    let result = super::transform_value_for_hash(value);
    assert!(
      !result.contains("100%"),
      "Expected 100%% to be removed from hash value but got: {}",
      result
    );
    assert!(
      result.ends_with("#0065ff)"),
      "Expected result to end with #0065ff) but got: {}",
      result
    );
  }

  #[test]
  fn transform_value_for_hash_handles_whitespace() {
    // This test matches Babel's cssnano behavior: 100% is removed but whitespace is preserved
    let value = "linear-gradient(90deg,\n\t\t\t#0065ff,\n\t\t\t#0469ff 12%,\n\t\t\t#bf63f3 24%,\n\t\t\t#ffa900 48%,\n\t\t\t#bf63f3 64%,\n\t\t\t#0469ff 80%,\n\t\t\t#0065ff 100%\n\t\t\t)";
    let result = super::transform_value_for_hash(value);
    assert!(
      !result.contains("100%"),
      "Expected 100%% to be removed from hash value with whitespace but got: {}",
      result
    );
    // Whitespace (newlines and tabs) should be preserved for correct hash computation
    assert!(
      result.contains("\n\t\t\t"),
      "Expected whitespace to be preserved but got: {}",
      result
    );
  }

  #[test]
  fn transform_value_for_hash_preserves_non_gradient_values() {
    let value = "red";
    let result = super::transform_value_for_hash(value);
    assert_eq!(result, "red");
  }
}
