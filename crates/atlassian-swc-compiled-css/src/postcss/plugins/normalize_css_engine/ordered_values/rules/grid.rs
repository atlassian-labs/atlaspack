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
  let has_span = parsed.nodes.iter().any(|node| match node {
    vp::Node::Word { value } => value.eq_ignore_ascii_case("span"),
    _ => false,
  });
  if !has_span {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
      if ch == '/' {
        out.push(ch);
        let mut ws = String::new();
        while let Some(&next) = chars.peek() {
          if next.is_whitespace() {
            ws.push(next);
            chars.next();
          } else {
            break;
          }
        }
        if let Some(&next) = chars.peek() {
          if next == '-' && ws == " " {
            // COMPAT: cssnano ordered-values preserves an extra space before
            // negative grid lines (e.g. "1 / -1" -> "1 /  -1").
            out.push_str("  ");
          } else {
            out.push_str(&ws);
          }
        } else {
          out.push_str(&ws);
        }
        continue;
      }
      out.push(ch);
    }
    let normalized = out.trim().to_string();
    return normalized;
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn normalize_line_preserves_negative_spacing_for_hash() {
    let parsed = vp::parse("1 / -1");
    assert_eq!(normalize_line(&parsed), "1 /  -1");
  }
}
