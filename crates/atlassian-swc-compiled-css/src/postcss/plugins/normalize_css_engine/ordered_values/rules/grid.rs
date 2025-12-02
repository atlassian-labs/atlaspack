use crate::postcss::value_parser as vp;

// Ports of src/rules/grid.js
pub fn normalize_auto_flow(parsed: &vp::ParsedValue) -> String {
  let mut front = String::new();
  let mut back = String::new();
  let mut should = false;
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        let v = value.trim().to_lowercase();
        if v == "dense" {
          should = true;
          back = value.clone();
        } else if v == "row" || v == "column" {
          should = true;
          front = value.clone();
        } else {
          should = false;
        }
      }
      true
    },
    false,
  );
  if should {
    format!("{} {}", front.trim(), back.trim())
  } else {
    vp::stringify(&parsed.nodes)
  }
}

pub fn normalize_gap(parsed: &vp::ParsedValue) -> String {
  let mut front = String::new();
  let mut back = String::new();
  let mut should = false;
  let mut nodes = parsed.nodes.clone();
  vp::walk(
    &mut nodes[..],
    &mut |node| {
      if let vp::Node::Word { value } = node {
        if value == "normal" {
          should = true;
          front = value.clone();
        } else {
          back.push(' ');
          back.push_str(value);
        }
      }
      true
    },
    false,
  );
  if should {
    format!("{} {}", front.trim(), back.trim())
  } else {
    vp::stringify(&parsed.nodes)
  }
}

pub fn normalize_line(parsed: &vp::ParsedValue) -> String {
  let s = vp::stringify(&parsed.nodes);
  let grid_value: Vec<String> = s.split('/').map(|p| p.to_string()).collect();
  if grid_value.len() > 1 {
    let mapped: Vec<String> = grid_value
      .into_iter()
      .map(|grid_line| {
        let mut front = String::new();
        let mut back = String::new();
        let gl = grid_line.trim().to_string();
        for token in gl.split(' ') {
          if token == "span" {
            front = token.to_string();
          } else {
            back.push(' ');
            back.push_str(token);
          }
        }
        format!("{} {}", front.trim(), back.trim())
      })
      .collect();
    return mapped.join(" / ").trim().to_string();
  }
  let mut out: Vec<String> = Vec::new();
  for grid_line in s.split('/') {
    let gl = grid_line.trim();
    let mut front = String::new();
    let mut back = String::new();
    for token in gl.split(' ') {
      if token == "span" {
        front = token.to_string();
      } else {
        back.push(' ');
        back.push_str(token);
      }
    }
    out.push(format!("{} {}", front.trim(), back.trim()));
  }
  out.join("")
}
