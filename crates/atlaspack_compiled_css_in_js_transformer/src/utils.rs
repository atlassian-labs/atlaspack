pub fn to_kebab_case(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for (i, ch) in s.chars().enumerate() {
    if ch.is_uppercase() {
      if i != 0 {
        out.push('-');
      }
      for lc in ch.to_lowercase() {
        out.push(lc);
      }
    } else {
      out.push(ch);
    }
  }
  out
}

pub fn normalize_at_query(query: &str) -> String {
  let mut q = query.trim().to_string();
  q = q.split_whitespace().collect::<Vec<_>>().join(" ");
  let mut out = String::with_capacity(q.len());
  let mut chars = q.chars().peekable();
  let mut prev: Option<char> = None;
  while let Some(ch) = chars.next() {
    if ch == '(' {
      out.push(ch);
      while let Some(' ') = chars.peek() {
        chars.next();
      }
    } else if ch == ')' {
      if matches!(prev, Some(' ')) {
        out.pop();
      }
      out.push(ch);
    } else if ch == ':' || ch == ',' {
      if matches!(prev, Some(' ')) {
        out.pop();
      }
      out.push(ch);
      while let Some(' ') = chars.peek() {
        chars.next();
      }
    } else {
      out.push(ch);
    }
    prev = out.chars().last();
  }
  out
}

pub fn normalize_nested_selector(sel: &str) -> String {
  let s = if sel.starts_with('&') {
    sel.to_string()
  } else {
    format!("&{}", sel)
  };
  let mut out = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == ':' {
      let mut colon_count = 1;
      if let Some(':') = chars.peek().copied() {
        colon_count += 1;
        chars.next();
      }
      let mut word = String::new();
      let mut tmp = chars.clone();
      while let Some(c2) = tmp.peek().copied() {
        if c2.is_alphanumeric() || c2 == '-' {
          word.push(c2);
          tmp.next();
        } else {
          break;
        }
      }
      if word == "after" || word == "before" {
        out.push(':');
      } else {
        if colon_count == 2 {
          out.push_str("::");
        } else {
          out.push(':');
        }
      }
    } else {
      out.push(ch);
    }
  }
  out
}
