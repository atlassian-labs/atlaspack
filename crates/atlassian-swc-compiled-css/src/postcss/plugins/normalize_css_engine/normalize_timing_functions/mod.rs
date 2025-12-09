use postcss as pc;

fn eq(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.0001
}

fn normalize_cubic_bezier(s: &str) -> Option<String> {
  let body = s.strip_prefix("cubic-bezier(")?.strip_suffix(")")?;
  let nums: Vec<f32> = body
    .split(',')
    .map(|p| p.trim())
    .filter_map(|t| t.parse::<f32>().ok())
    .collect();
  if nums.len() != 4 {
    return None;
  }
  let (a, b, c, d) = (nums[0], nums[1], nums[2], nums[3]);
  if eq(a, 0.25) && eq(b, 0.1) && eq(c, 0.25) && eq(d, 1.0) {
    return Some("ease".to_string());
  }
  if eq(a, 0.0) && eq(b, 0.0) && eq(c, 1.0) && eq(d, 1.0) {
    return Some("linear".to_string());
  }
  if eq(a, 0.42) && eq(b, 0.0) && eq(c, 1.0) && eq(d, 1.0) {
    return Some("ease-in".to_string());
  }
  if eq(a, 0.0) && eq(b, 0.0) && eq(c, 0.58) && eq(d, 1.0) {
    return Some("ease-out".to_string());
  }
  if eq(a, 0.42) && eq(b, 0.0) && eq(c, 0.58) && eq(d, 1.0) {
    return Some("ease-in-out".to_string());
  }
  None
}

fn normalize_steps(s: &str) -> Option<String> {
  let body = s.strip_prefix("steps(")?.strip_suffix(")")?;
  let parts: Vec<&str> = body.split(',').map(|p| p.trim()).collect();
  if parts.len() == 2 && parts[0] == "1" {
    let kw = parts[1].to_ascii_lowercase();
    if kw == "start" {
      return Some("step-start".to_string());
    }
    if kw == "end" {
      return Some("step-end".to_string());
    }
  }
  None
}

pub fn plugin() -> pc::BuiltPlugin {
  pc::plugin("postcss-normalize-timing-functions")
    .decl(|decl, _| {
      let current = decl.value();
      let mut next = current.clone();
      // cubic-bezier
      let mut i = 0usize;
      while let Some(start) = next[i..].find("cubic-bezier(") {
        let idx = i + start;
        if let Some(end_rel) = next[idx..].find(')') {
          let end = idx + end_rel + 1;
          let seg = &next[idx..end];
          if let Some(name) = normalize_cubic_bezier(seg) {
            next.replace_range(idx..end, &name);
            i = idx + name.len();
            continue;
          }
          i = end;
        } else {
          break;
        }
      }
      // steps
      let mut j = 0usize;
      while let Some(start) = next[j..].find("steps(") {
        let idx = j + start;
        if let Some(end_rel) = next[idx..].find(')') {
          let end = idx + end_rel + 1;
          let seg = &next[idx..end];
          if let Some(name) = normalize_steps(seg) {
            next.replace_range(idx..end, &name);
            j = idx + name.len();
            continue;
          }
          j = end;
        } else {
          break;
        }
      }
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .build()
}
