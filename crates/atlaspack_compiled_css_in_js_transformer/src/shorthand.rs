use crate::evaluate_expr::ValueOrNumber;

// Minimal keyword set for border/outline styles
const BORDER_STYLES: &[&str] = &[
  "none", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];

fn is_border_style(token: &str) -> bool {
  let lower = token.to_ascii_lowercase();
  BORDER_STYLES.contains(&lower.as_str())
}

fn is_color_token(token: &str) -> bool {
  let lower = token.to_ascii_lowercase();
  if lower.starts_with('#') {
    return true;
  }
  if lower == "transparent" || lower == "currentcolor" {
    return true;
  }
  // Heuristic: functional tokens are likely colors (rgb(), hsl(), color(), var(), etc.)
  if lower.contains('(') {
    return true;
  }
  false
}

fn is_length_token(token: &str) -> bool {
  if token == "0" {
    return true;
  }
  let t = token.trim();
  if t.is_empty() {
    return false;
  }
  // Parse a number prefix (optional sign, digits, optional decimal)
  let mut i = 0usize;
  let bytes = t.as_bytes();
  if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
    i += 1;
  }
  let mut has_digit = false;
  while i < bytes.len() && bytes[i].is_ascii_digit() {
    has_digit = true;
    i += 1;
  }
  if i < bytes.len() && bytes[i] == b'.' {
    i += 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
      has_digit = true;
      i += 1;
    }
  }
  if !has_digit {
    return false;
  }
  // Remaining suffix: any alphabetic sequence or % counts as a unit
  if i >= bytes.len() {
    return false;
  }
  let suffix = &t[i..];
  if suffix == "%" {
    return true;
  }
  suffix.chars().all(|c| c.is_ascii_alphabetic())
}

fn parse_border_outline_tokens(
  input: &str,
  number_only: bool,
) -> (Option<String>, Option<String>, Option<String>) {
  // returns (width, style, color)
  if number_only {
    // treat as width only
    return (Some("px".into()), None, None);
  }
  let mut width: Option<String> = None;
  let mut style: Option<String> = None;
  let mut color: Option<String> = None;
  for tok in input.split_whitespace() {
    if width.is_none() && is_length_token(tok) {
      width = Some(tok.to_string());
      continue;
    }
    if style.is_none() && is_border_style(tok) {
      style = Some(tok.to_string());
      continue;
    }
    if color.is_none() && is_color_token(tok) {
      color = Some(tok.to_string());
      continue;
    }
    // If token didn't match any, attempt heuristics:
    if width.is_none() && tok.chars().all(|c| c.is_ascii_digit()) {
      width = Some(format!("{}px", tok));
      continue;
    }
    // treat remaining unknown token as color (named colors like green, blue, etc.)
    if color.is_none() {
      color = Some(tok.to_string());
      continue;
    }
  }
  (width, style, color)
}

fn side_suffix(prop: &str) -> Option<&'static str> {
  if prop.starts_with("border-top") {
    Some("-top")
  } else if prop.starts_with("border-right") {
    Some("-right")
  } else if prop.starts_with("border-bottom") {
    Some("-bottom")
  } else if prop.starts_with("border-left") {
    Some("-left")
  } else {
    None
  }
}

pub fn try_expand_shorthand(
  prop_kebab: &str,
  value: &ValueOrNumber,
) -> Option<Vec<(String, String)>> {
  match prop_kebab {
    // padding and margin: 1-4 values map to each side
    "padding" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let vals = expand_1_to_4(&tokens.join(" "));
      Some(vec![
        ("padding-top".into(), vals[0].clone()),
        ("padding-right".into(), vals[1].clone()),
        ("padding-bottom".into(), vals[2].clone()),
        ("padding-left".into(), vals[3].clone()),
      ])
    }
    "margin" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let vals = expand_1_to_4(&tokens.join(" "));
      Some(vec![
        ("margin-top".into(), vals[0].clone()),
        ("margin-right".into(), vals[1].clone()),
        ("margin-bottom".into(), vals[2].clone()),
        ("margin-left".into(), vals[3].clone()),
      ])
    }
    // outline: Only emit width to preserve existing fixture behavior
    "outline" => {
      let width = match value {
        ValueOrNumber::Num(n) => Some(format!("{}px", trim_float(*n))),
        ValueOrNumber::Str(s) => parse_border_outline_tokens(s, false).0,
      };
      let mut out = Vec::new();
      if let Some(w) = width {
        out.push(("outline-width".into(), w));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    p if p == "border"
      || p.starts_with("border-top")
      || p.starts_with("border-right")
      || p.starts_with("border-bottom")
      || p.starts_with("border-left") =>
    {
      let (mut width, mut style, mut color) = match value {
        ValueOrNumber::Num(n) => (Some(format!("{}px", trim_float(*n))), None, None),
        ValueOrNumber::Str(s) => parse_border_outline_tokens(s, false),
      };
      // If it's style-only (e.g., "none") and no width/color, keep shorthand to match legacy/basic cases
      if width.is_none() && color.is_none() {
        if let Some(st) = &style {
          let st_lc = st.to_ascii_lowercase();
          if st_lc == "none"
            || st_lc == "hidden"
            || st_lc == "dotted"
            || st_lc == "dashed"
            || st_lc == "solid"
            || st_lc == "double"
            || st_lc == "groove"
            || st_lc == "ridge"
            || st_lc == "inset"
            || st_lc == "outset"
          {
            return None;
          }
        }
      }
      let base = if let Some(suf) = side_suffix(prop_kebab) {
        format!("border{}", suf)
      } else {
        "border".into()
      };
      let mut out = Vec::new();
      if let Some(w) = width.take() {
        out.push((format!("{}-width", base), w));
      }
      if let Some(st) = style.take() {
        out.push((format!("{}-style", base), st));
      }
      if let Some(c) = color.take() {
        out.push((format!("{}-color", base), c));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // border-color/style/width: 1-4 values map to each side
    p @ ("border-color" | "border-style" | "border-width") => {
      let tokens = val_tokens(value);
      let vals = expand_1_to_4(&tokens.join(" "));
      let suffix = if p.ends_with("color") {
        "color"
      } else if p.ends_with("style") {
        "style"
      } else {
        "width"
      };
      let mut out = Vec::new();
      out.push((format!("border-top-{}", suffix), vals[0].clone()));
      out.push((format!("border-right-{}", suffix), vals[1].clone()));
      out.push((format!("border-bottom-{}", suffix), vals[2].clone()));
      out.push((format!("border-left-{}", suffix), vals[3].clone()));
      Some(out)
    }
    // overflow: one or two tokens -> overflow-x, overflow-y
    "overflow" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![("overflow-x".into(), a), ("overflow-y".into(), b)])
    }
    // columns: <column-width> || <column-count>
    "columns" => {
      let tokens = val_tokens(value);
      let mut count: Option<String> = None;
      let mut width: Option<String> = None;
      for tok in tokens {
        if count.is_none() && tok.chars().all(|c| c.is_ascii_digit()) {
          count = Some(tok);
          continue;
        }
        if width.is_none() && (is_length_token(&tok) || tok == "auto") {
          width = Some(tok);
          continue;
        }
      }
      let mut out = Vec::new();
      if let Some(c) = count.take() {
        out.push(("column-count".into(), c));
      }
      if let Some(w) = width.take() {
        out.push(("column-width".into(), w));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // gap: row-gap column-gap
    "gap" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![("row-gap".into(), a), ("column-gap".into(), b)])
    }
    // column-rule: width style color
    "column-rule" => {
      let (width, style, color) = match value {
        ValueOrNumber::Num(n) => (Some(format!("{}px", trim_float(*n))), None, None),
        ValueOrNumber::Str(s) => parse_border_outline_tokens(s, false),
      };
      let mut out = Vec::new();
      if let Some(w) = width {
        out.push(("column-rule-width".into(), w));
      }
      if let Some(st) = style {
        out.push(("column-rule-style".into(), st));
      }
      if let Some(c) = color {
        out.push(("column-rule-color".into(), c));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // text-decoration: [line] [style] [color] [thickness]
    "text-decoration" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut line: Option<String> = None;
      let mut style: Option<String> = None;
      let mut color: Option<String> = None;
      let mut thickness: Option<String> = None;
      for tok in tokens {
        let l = tok.to_ascii_lowercase();
        if line.is_none()
          && (l == "none" || l == "underline" || l == "overline" || l == "line-through")
        {
          line = Some(tok);
          continue;
        }
        if style.is_none()
          && (l == "solid" || l == "double" || l == "dotted" || l == "dashed" || l == "wavy")
        {
          style = Some(tok);
          continue;
        }
        if color.is_none() && is_color_token(&l) {
          color = Some(tok);
          continue;
        }
        if thickness.is_none() && (is_length_token(&l) || l == "auto" || l == "from-font") {
          thickness = Some(tok);
          continue;
        }
      }
      let mut out = Vec::new();
      if let Some(v) = line {
        out.push(("text-decoration-line".into(), v));
      }
      if let Some(v) = style {
        out.push(("text-decoration-style".into(), v));
      }
      if let Some(v) = color {
        out.push(("text-decoration-color".into(), v));
      }
      if let Some(v) = thickness {
        out.push(("text-decoration-thickness".into(), v));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // list-style: [list-style-type] [position] [image]
    "list-style" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut list_type: Option<String> = None;
      let mut position: Option<String> = None;
      let mut image: Option<String> = None;
      for tok in tokens {
        let l = tok.to_ascii_lowercase();
        if position.is_none() && (l == "inside" || l == "outside") {
          position = Some(tok);
          continue;
        }
        if image.is_none() && (l.starts_with("url(") || l == "none") {
          image = Some(tok);
          continue;
        }
        if list_type.is_none() {
          list_type = Some(tok);
          continue;
        }
      }
      let mut out = Vec::new();
      if let Some(v) = image {
        out.push(("list-style-image".into(), v));
      }
      if let Some(v) = position {
        out.push(("list-style-position".into(), v));
      }
      if let Some(v) = list_type {
        out.push(("list-style-type".into(), v));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // scroll-margin: 1-4 values => top right bottom left
    "scroll-margin" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let vals = expand_1_to_4(&tokens.join(" "));
      Some(vec![
        ("scroll-margin-top".into(), vals[0].clone()),
        ("scroll-margin-right".into(), vals[1].clone()),
        ("scroll-margin-bottom".into(), vals[2].clone()),
        ("scroll-margin-left".into(), vals[3].clone()),
      ])
    }
    // scroll-margin-block/inline: 1-2 values -> start/end
    p @ ("scroll-margin-block" | "scroll-margin-inline") => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      // We append only -start/-end because p already contains -block or -inline
      Some(vec![(format!("{}-start", p), a), (format!("{}-end", p), b)])
    }
    // scroll-padding: 1-4 values => top right bottom left
    "scroll-padding" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let vals = expand_1_to_4(&tokens.join(" "));
      Some(vec![
        ("scroll-padding-top".into(), vals[0].clone()),
        ("scroll-padding-right".into(), vals[1].clone()),
        ("scroll-padding-bottom".into(), vals[2].clone()),
        ("scroll-padding-left".into(), vals[3].clone()),
      ])
    }
    // scroll-padding-block/inline: 1-2 values -> start/end
    p @ ("scroll-padding-block" | "scroll-padding-inline") => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      // We append only -start/-end because p already contains -block or -inline
      Some(vec![(format!("{}-start", p), a), (format!("{}-end", p), b)])
    }
    // overscroll-behavior: one or two tokens -> x/y
    "overscroll-behavior" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![
        ("overscroll-behavior-x".into(), a),
        ("overscroll-behavior-y".into(), b),
      ])
    }
    // place-content/items/self: 1-2 values -> map to align/justify counterparts
    "place-content" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![
        ("align-content".into(), a),
        ("justify-content".into(), b),
      ])
    }
    "place-items" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![("align-items".into(), a), ("justify-items".into(), b)])
    }
    "place-self" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![("align-self".into(), a), ("justify-self".into(), b)])
    }
    // text-wrap: mode style (up to two tokens)
    "text-wrap" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut out = Vec::new();
      if tokens.len() >= 1 {
        out.push(("text-wrap-mode".into(), tokens[0].clone()));
      }
      if tokens.len() >= 2 {
        out.push(("text-wrap-style".into(), tokens[1].clone()));
      }
      Some(out)
    }
    // text-emphasis: [style] [color]
    "text-emphasis" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut style: Option<String> = None;
      let mut color: Option<String> = None;
      for tok in tokens {
        let l = tok.to_ascii_lowercase();
        if color.is_none() && is_color_token(&l) {
          color = Some(tok);
          continue;
        }
        if style.is_none() {
          style = Some(tok);
          continue;
        }
      }
      let mut out = Vec::new();
      if let Some(c) = color {
        out.push(("text-emphasis-color".into(), c));
      }
      if let Some(s) = style {
        out.push(("text-emphasis-style".into(), s));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // inset-block and inset-inline: 1-2 values -> start/end
    p @ ("inset-block" | "inset-inline") => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      let (start, end) = if p.ends_with("inline") {
        ("inline-start", "inline-end")
      } else {
        ("block-start", "block-end")
      };
      Some(vec![
        (format!("{}-{}", p, start), a),
        (format!("{}-{}", p, end), b),
      ])
    }
    // flex-flow: <flex-direction> <flex-wrap>
    "flex-flow" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut direction: Option<String> = None;
      let mut wrap: Option<String> = None;
      for tok in tokens {
        let l = tok.to_ascii_lowercase();
        if direction.is_none()
          && (l == "row" || l == "row-reverse" || l == "column" || l == "column-reverse")
        {
          direction = Some(tok);
          continue;
        }
        if wrap.is_none() && (l == "nowrap" || l == "wrap" || l == "wrap-reverse") {
          wrap = Some(tok);
          continue;
        }
      }
      let mut out = Vec::new();
      if let Some(d) = direction {
        out.push(("flex-direction".into(), d));
      }
      if let Some(w) = wrap {
        out.push(("flex-wrap".into(), w));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // grid-column and grid-row: <start> / <end>
    p @ ("grid-column" | "grid-row") => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let joined = tokens.join(" ");
      let mut parts = joined
        .split('/')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();
      if parts.is_empty() {
        return None;
      }
      if parts.len() == 1 {
        parts.push(parts[0].clone());
      }
      let (start_prop, end_prop) = if p == "grid-column" {
        ("grid-column-start", "grid-column-end")
      } else {
        ("grid-row-start", "grid-row-end")
      };
      Some(vec![
        (start_prop.into(), parts[0].clone()),
        (end_prop.into(), parts[1].clone()),
      ])
    }
    // grid-area: <row-start> / <column-start> / <row-end> / <column-end>
    "grid-area" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let parts = tokens.join(" ");
      let items = parts
        .split('/')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();
      if items.is_empty() {
        return None;
      }
      let mut out = Vec::new();
      if items.len() >= 1 {
        out.push(("grid-row-start".into(), items[0].clone()));
      }
      if items.len() >= 2 {
        out.push(("grid-column-start".into(), items[1].clone()));
      }
      if items.len() >= 3 {
        out.push(("grid-row-end".into(), items[2].clone()));
      }
      if items.len() >= 4 {
        out.push(("grid-column-end".into(), items[3].clone()));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // animation-range: <start> <end>
    "animation-range" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      Some(vec![
        ("animation-range-start".into(), a),
        ("animation-range-end".into(), b),
      ])
    }
    // position-try: <order> <fallbacks>
    "position-try" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut out = Vec::new();
      if tokens.len() >= 1 {
        out.push(("position-try-order".into(), tokens[0].clone()));
      }
      if tokens.len() >= 2 {
        out.push(("position-try-fallbacks".into(), tokens[1].clone()));
      }
      Some(out)
    }
    // container: name type (heuristic)
    "container" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut name: Option<String> = None;
      let mut ctype: Option<String> = None;
      for t in tokens {
        if ctype.is_none()
          && (t == "normal" || t.ends_with("-size") || t == "inline-size" || t == "size")
        {
          ctype = Some(t);
          continue;
        }
        if name.is_none() {
          name = Some(t);
        }
      }
      let mut out = Vec::new();
      if let Some(n) = name {
        out.push(("container-name".into(), n));
      }
      if let Some(ct) = ctype {
        out.push(("container-type".into(), ct));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // contain-intrinsic-size
    "contain-intrinsic-size" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      if tokens.len() == 2 {
        return Some(vec![
          ("contain-intrinsic-block-size".into(), tokens[0].clone()),
          ("contain-intrinsic-inline-size".into(), tokens[1].clone()),
        ]);
      }
      Some(vec![
        ("contain-intrinsic-block-size".into(), tokens[0].clone()),
        ("contain-intrinsic-inline-size".into(), tokens[0].clone()),
      ])
    }
    // scroll-timeline
    "scroll-timeline" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut name: Option<String> = None;
      let mut axis: Option<String> = None;
      for t in tokens {
        let l = t.to_ascii_lowercase();
        if axis.is_none() && (l == "x" || l == "y" || l == "inline" || l == "block") {
          axis = Some(t);
          continue;
        }
        if name.is_none() {
          name = Some(t);
        }
      }
      let mut out = Vec::new();
      if let Some(n) = name {
        out.push(("scroll-timeline-name".into(), n));
      }
      if let Some(a) = axis {
        out.push(("scroll-timeline-axis".into(), a));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // view-timeline
    "view-timeline" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let mut name: Option<String> = None;
      let mut axis: Option<String> = None;
      for t in tokens {
        let l = t.to_ascii_lowercase();
        if axis.is_none() && (l == "x" || l == "y" || l == "inline" || l == "block") {
          axis = Some(t);
          continue;
        }
        if name.is_none() {
          name = Some(t);
        }
      }
      let mut out = Vec::new();
      if let Some(n) = name {
        out.push(("view-timeline-name".into(), n));
      }
      if let Some(a) = axis {
        out.push(("view-timeline-axis".into(), a));
      }
      if out.is_empty() { None } else { Some(out) }
    }
    // inset: 1-4 values => top right bottom left
    "inset" => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let vals = expand_1_to_4(&tokens.join(" "));
      Some(vec![
        ("top".into(), vals[0].clone()),
        ("right".into(), vals[1].clone()),
        ("bottom".into(), vals[2].clone()),
        ("left".into(), vals[3].clone()),
      ])
    }
    // margin-inline/block and padding-inline/block: 1-2 values -> start/end
    p @ ("margin-inline" | "padding-inline" | "margin-block" | "padding-block") => {
      let tokens = val_tokens(value);
      if tokens.is_empty() {
        return None;
      }
      let a = tokens[0].clone();
      let b = if tokens.len() > 1 {
        tokens[1].clone()
      } else {
        a.clone()
      };
      let (start, end) = if p.ends_with("inline") {
        ("-inline-start", "-inline-end")
      } else {
        ("-block-start", "-block-end")
      };
      Some(vec![
        (
          format!(
            "{}{}",
            p.trim_end_matches("-inline").trim_end_matches("-block"),
            start
          ),
          a,
        ),
        (
          format!(
            "{}{}",
            p.trim_end_matches("-inline").trim_end_matches("-block"),
            end
          ),
          b,
        ),
      ])
    }
    // padding-inline-start etc are not shorthands, skip
    // Do not expand border-radius; keep as a single property to match babel parity
    _ => None,
  }
}

/// Returns true if the provided kebab-case property name is a shorthand that
/// we know how to expand. This is used by the template literal parser to avoid
/// allocating a `String` for values when it's not necessary.
pub fn is_shorthand_prop(prop_kebab: &str) -> bool {
  match prop_kebab {
    // 1-to-4 value shorthands
    "padding" | "margin" | "scroll-margin" | "scroll-padding" | "inset" => true,
    // axis/block/inline pairs
    "scroll-margin-block"
    | "scroll-margin-inline"
    | "scroll-padding-block"
    | "scroll-padding-inline" => true,
    // border compound
    p if p == "border"
      || p.starts_with("border-top")
      || p.starts_with("border-right")
      || p.starts_with("border-bottom")
      || p.starts_with("border-left") =>
    {
      true
    }
    // border triads
    "border-color" | "border-style" | "border-width" => true,
    // outline minimal handling
    "outline" => true,
    // misc multi-token shorthands
    "overflow" | "columns" | "gap" | "column-rule" | "text-decoration" | "list-style" => true,
    // 2-part place-* and overscroll-behavior
    "place-content" | "place-items" | "place-self" | "overscroll-behavior" => true,
    // text wrap/emphasis
    "text-wrap" | "text-emphasis" => true,
    // logical margin/padding pairs
    "margin-inline" | "padding-inline" | "margin-block" | "padding-block" => true,
    // flex/grid
    "flex-flow" | "grid-column" | "grid-row" | "grid-area" => true,
    // animation/position/container families
    "animation-range" | "position-try" | "container" | "contain-intrinsic-size" => true,
    // timeline
    "scroll-timeline" | "view-timeline" => true,
    _ => false,
  }
}

fn trim_float(n: f64) -> String {
  if (n - (n as i64 as f64)).abs() < f64::EPSILON {
    (n as i64).to_string()
  } else {
    n.to_string()
  }
}

fn val_tokens(value: &ValueOrNumber) -> Vec<String> {
  match value {
    ValueOrNumber::Num(n) => vec![trim_float(*n) + "px"],
    ValueOrNumber::Str(s) => s.split_whitespace().map(|t| t.to_string()).collect(),
  }
}

fn expand_1_to_4(input: &str) -> [String; 4] {
  let parts: Vec<&str> = input.split_whitespace().collect();
  match parts.len() {
    0 => ["".into(), "".into(), "".into(), "".into()],
    1 => {
      let v = parts[0].to_string();
      [v.clone(), v.clone(), v.clone(), v]
    }
    2 => {
      let v_tb = parts[0].to_string();
      let v_lr = parts[1].to_string();
      [v_tb.clone(), v_lr.clone(), v_tb, v_lr]
    }
    3 => {
      let v_t = parts[0].to_string();
      let v_lr = parts[1].to_string();
      let v_b = parts[2].to_string();
      [v_t, v_lr.clone(), v_b, v_lr]
    }
    _ => [
      parts[0].to_string(),
      parts[1].to_string(),
      parts[2].to_string(),
      parts[3].to_string(),
    ],
  }
}
