#[cfg(feature = "postcss_engine")]
use postcss as pc;

// Submodules mirroring JS plugin file structure for clarity and parity.
pub(crate) mod browserslist_support;
pub mod calc;
pub mod colormin;
pub mod convert_values;
pub mod minify_gradients;
pub mod minify_params;
pub mod minify_selectors;
pub mod normalize_positions;
pub mod normalize_string;
pub mod normalize_timing_functions;
pub mod normalize_unicode;
pub mod normalize_url;
pub mod ordered_values;
pub mod reduce_initial;

#[cfg(feature = "postcss_engine")]
fn is_whitespace(ch: char) -> bool {
  matches!(ch, ' ' | '\n' | '\t' | '\r' | '\x0C')
}

/// Collapse whitespace in a CSS selector while preserving semantics.
/// This approximates postcss-minify-selectors@5 behaviour for common cases:
/// - Remove spaces around combinators `> + ~ ,`
/// - Collapse multiple spaces to a single descendant space
/// - Preserve content inside quotes, attribute selectors, and parentheses
#[cfg(feature = "postcss_engine")]
pub fn minify_selector_whitespace(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut in_single = false;
  let mut in_double = false;
  let mut escape_next = false;
  let mut bracket_depth = 0usize; // [ ]
  let mut paren_depth = 0usize; // ( )

  // Helper to minify the inside of an attribute selector: [ ... ]
  fn minify_attr(content: &str) -> String {
    // Remove spaces around attribute operators and collapse other spaces
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    let chars: Vec<char> = content.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    while i < chars.len() {
      let ch = chars[i];
      if escape {
        out.push(ch);
        escape = false;
        i += 1;
        continue;
      }
      if ch == '\\' {
        escape = true;
        out.push(ch);
        i += 1;
        continue;
      }
      if in_single {
        out.push(ch);
        if ch == '\'' {
          in_single = false;
        }
        i += 1;
        continue;
      }
      if in_double {
        out.push(ch);
        if ch == '"' {
          in_double = false;
        }
        i += 1;
        continue;
      }
      match ch {
        '\'' => {
          in_single = true;
          out.push(ch);
        }
        '"' => {
          in_double = true;
          out.push(ch);
        }
        c if is_whitespace(c) => {
          // Lookahead to next non-space
          let mut j = i + 1;
          while j < chars.len() && is_whitespace(chars[j]) {
            j += 1;
          }
          let prev = out.chars().rev().find(|c| !is_whitespace(*c));
          let next = chars.get(j).copied();
          let around_op = |p: Option<char>, n: Option<char>| {
            let pair = match (p, n) {
              (Some('^'), Some('='))
              | (Some('$'), Some('='))
              | (Some('*'), Some('='))
              | (Some('|'), Some('='))
              | (Some('~'), Some('=')) => true,
              (Some('='), _) | (_, Some('=')) => true,
              _ => false,
            };
            pair
          };
          if around_op(prev, next) { /* drop */
          } else {
            out.push(' ');
          }
          i = j;
          continue;
        }
        _ => out.push(ch),
      }
      i += 1;
    }
    // Trim
    let s = out.trim().to_string();
    // Also remove spaces directly inside brackets like "[ attr ]" -> "[attr]"
    s.replace(" =", "=").replace("= ", "=")
  }

  // Helper to decide if we should retain a space at position i
  let bytes: Vec<char> = input.chars().collect();
  let mut i = 0usize;
  while i < bytes.len() {
    let ch = bytes[i];
    if escape_next {
      out.push(ch);
      escape_next = false;
      i += 1;
      continue;
    }
    if ch == '\\' {
      escape_next = true;
      out.push(ch);
      i += 1;
      continue;
    }
    if in_single {
      out.push(ch);
      if ch == '\'' {
        in_single = false;
      }
      i += 1;
      continue;
    }
    if in_double {
      out.push(ch);
      if ch == '"' {
        in_double = false;
      }
      i += 1;
      continue;
    }
    match ch {
      '\'' => {
        in_single = true;
        out.push(ch);
      }
      '"' => {
        in_double = true;
        out.push(ch);
      }
      '[' => {
        bracket_depth += 1;
        // Capture until matching ']'
        out.push('[');
        let mut j = i + 1;
        let mut depth = 1usize;
        let mut s_in_single = false;
        let mut s_in_double = false;
        let mut s_escape = false;
        let start = j;
        while j < bytes.len() {
          let c = bytes[j];
          if s_escape {
            s_escape = false;
            j += 1;
            continue;
          }
          if c == '\\' {
            s_escape = true;
            j += 1;
            continue;
          }
          if s_in_single {
            if c == '\'' {
              s_in_single = false;
            }
            j += 1;
            continue;
          }
          if s_in_double {
            if c == '"' {
              s_in_double = false;
            }
            j += 1;
            continue;
          }
          match c {
            '\'' => s_in_single = true,
            '"' => s_in_double = true,
            '[' => depth += 1,
            ']' => {
              depth -= 1;
              if depth == 0 {
                break;
              }
            }
            _ => {}
          }
          j += 1;
        }
        let inner = bytes[start..j].iter().collect::<String>();
        let minimized = minify_attr(&inner);
        out.push_str(&minimized);
        out.push(']');
        // advance to after ']'
        i = if j < bytes.len() { j + 1 } else { j };
        bracket_depth -= 1; // closed
        continue;
      }
      ']' => {
        if bracket_depth > 0 {
          bracket_depth -= 1;
        }
        out.push(ch);
      }
      '(' => {
        paren_depth += 1;
        out.push(ch);
      }
      ')' => {
        if paren_depth > 0 {
          paren_depth -= 1;
        }
        out.push(ch);
      }
      ch if is_whitespace(ch) => {
        // Look ahead/back to decide collapsing/removal
        // Skip all consecutive whitespace
        let mut j = i + 1;
        while j < bytes.len() && is_whitespace(bytes[j]) {
          j += 1;
        }
        let prev = out.chars().rev().find(|c| !is_whitespace(*c));
        let next = bytes.get(j).copied();

        // If around combinators or commas, drop space
        let is_combinator =
          |c: Option<char>| matches!(c, Some('>') | Some('+') | Some('~') | Some(','));
        if is_combinator(prev) || is_combinator(next) {
          // remove
        } else if bracket_depth > 0 {
          // Inside attribute selector: collapse to single space
          out.push(' ');
        } else if paren_depth > 0 {
          // Inside :not() etc.: collapse to single space
          out.push(' ');
        } else {
          // Descendant combinator: single space
          out.push(' ');
        }
        i = j;
        continue;
      }
      // Remove spaces around combinators
      '>' | '+' | '~' | ',' => {
        // Trim any trailing space in output
        while out.ends_with(' ') {
          out.pop();
        }
        out.push(ch);
        // Skip following spaces
        let mut j = i + 1;
        while j < bytes.len() && is_whitespace(bytes[j]) {
          j += 1;
        }
        i = j;
        continue;
      }
      _ => out.push(ch),
    }
    i += 1;
  }
  out.trim().to_string()
}

#[cfg(feature = "postcss_engine")]
pub fn ordered_values_plugin() -> pc::BuiltPlugin {
  // Delegate to structured port if available; fallback to existing implementation below
  #[allow(unused_mut)]
  let mut use_new = true;
  if use_new {
    return self::ordered_values::plugin();
  }
  use postcss::list::{comma, space};
  fn minimize_box_shorthand(value: &str) -> String {
    let parts = space(value);
    if parts.is_empty() {
      return value.to_string();
    }
    // If any part contains a comma, multiple layers present - bail
    if value.contains(',') {
      return value.to_string();
    }
    // Normalize to 4 values
    let mut vals: Vec<String> = match parts.len() {
      1 => vec![
        parts[0].clone(),
        parts[0].clone(),
        parts[0].clone(),
        parts[0].clone(),
      ],
      2 => vec![
        parts[0].clone(),
        parts[1].clone(),
        parts[0].clone(),
        parts[1].clone(),
      ],
      3 => vec![
        parts[0].clone(),
        parts[1].clone(),
        parts[2].clone(),
        parts[1].clone(),
      ],
      _ => parts.clone(),
    };
    if vals.len() < 4 {
      vals.resize(4, vals.last().cloned().unwrap_or_default());
    }
    let (top, right, bottom, left) = (&vals[0], &vals[1], &vals[2], &vals[3]);
    // Reduce
    if top == right && right == bottom && bottom == left {
      return top.clone();
    }
    if top == bottom && right == left {
      return format!("{} {}", top, right);
    }
    if right == left {
      return format!("{} {} {}", top, right, bottom);
    }
    format!("{} {} {} {}", top, right, bottom, left)
  }

  fn is_time_token(tok: &str) -> bool {
    // duration/delay tokens: 1s, .2s, 200ms
    let lower = tok.to_lowercase();
    (lower.ends_with("ms") || lower.ends_with('s')) && lower.chars().any(|c| c.is_ascii_digit())
  }

  fn is_timing_function(tok: &str) -> bool {
    let lower = tok.to_lowercase();
    matches!(
      lower.as_str(),
      "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out" | "step-start" | "step-end"
    ) || lower.starts_with("cubic-bezier(")
      || lower.starts_with("steps(")
  }

  fn is_integer(tok: &str) -> bool {
    tok.chars().all(|c| c.is_ascii_digit())
  }

  fn canonicalize_border_outline(value: &str) -> String {
    // Canonical order: width style color
    let mut width: Option<String> = None;
    let mut style: Option<String> = None;
    let mut color: Option<String> = None;
    for t in space(value) {
      let lower = t.to_lowercase();
      // Width keywords/units
      let is_width_kw = matches!(lower.as_str(), "thin" | "medium" | "thick")
        || lower.chars().any(|c| c.is_ascii_digit());
      // Style keywords
      let is_style = matches!(
        lower.as_str(),
        "none"
          | "hidden"
          | "dotted"
          | "dashed"
          | "solid"
          | "double"
          | "groove"
          | "ridge"
          | "inset"
          | "outset"
      );
      // Color heuristics: functions/hex/named
      let is_color = lower.starts_with('#') || lower.ends_with(')') || (!is_style && !is_width_kw);
      if is_style && style.is_none() {
        style = Some(t);
      } else if is_width_kw && width.is_none() {
        width = Some(t);
      } else if is_color && color.is_none() {
        color = Some(t);
      }
    }
    let mut out: Vec<String> = Vec::new();
    if let Some(w) = width {
      out.push(w);
    }
    if let Some(s) = style {
      out.push(s);
    }
    if let Some(c) = color {
      out.push(c);
    }
    if out.is_empty() {
      value.to_string()
    } else {
      out.join(" ")
    }
  }

  fn canonicalize_list_style(value: &str) -> String {
    // Order: type position image
    let mut ty: Option<String> = None; // disc/circle/square/decimal/none/…
    let mut pos: Option<String> = None; // inside/outside
    let mut img: Option<String> = None; // url(...) / none
    for t in space(value) {
      let lower = t.to_lowercase();
      if matches!(lower.as_str(), "inside" | "outside") && pos.is_none() {
        pos = Some(t);
      } else if lower.starts_with("url(") && img.is_none() {
        img = Some(t);
      } else if ty.is_none() {
        ty = Some(t);
      }
    }
    let mut out: Vec<String> = Vec::new();
    if let Some(t) = ty {
      out.push(t);
    }
    if let Some(p) = pos {
      out.push(p);
    }
    if let Some(i) = img {
      out.push(i);
    }
    if out.is_empty() {
      value.to_string()
    } else {
      out.join(" ")
    }
  }

  fn canonicalize_flex_flow(value: &str) -> String {
    // Order: <flex-direction> <flex-wrap>
    let mut dir: Option<String> = None;
    let mut wrap: Option<String> = None;
    for t in space(value) {
      let lower = t.to_lowercase();
      if matches!(lower.as_str(), "nowrap" | "wrap" | "wrap-reverse") && wrap.is_none() {
        wrap = Some(t);
      } else if dir.is_none() {
        dir = Some(t);
      }
    }
    let mut out: Vec<String> = Vec::new();
    if let Some(d) = dir {
      out.push(d);
    }
    if let Some(w) = wrap {
      out.push(w);
    }
    if out.is_empty() {
      value.to_string()
    } else {
      out.join(" ")
    }
  }

  fn canonicalize_transition(value: &str) -> String {
    // Each item: <property> <duration> <timing-function> <delay>
    let mut items: Vec<String> = Vec::new();
    for item in comma(value) {
      let mut property: Option<String> = None;
      let mut duration: Option<String> = None;
      let mut delay: Option<String> = None;
      let mut timing: Option<String> = None;
      for t in space(&item) {
        let lower = t.to_lowercase();
        if is_time_token(&lower) {
          if duration.is_none() {
            duration = Some(t);
          } else if delay.is_none() {
            delay = Some(t);
          }
        } else if is_timing_function(&lower) {
          if timing.is_none() {
            timing = Some(t);
          }
        } else if property.is_none() {
          property = Some(t);
        }
      }
      let mut out: Vec<String> = Vec::new();
      if let Some(p) = property {
        out.push(p);
      }
      if let Some(d) = duration {
        out.push(d);
      }
      if let Some(tf) = timing {
        out.push(tf);
      }
      if let Some(dl) = delay {
        out.push(dl);
      }
      if out.is_empty() {
        items.push(item);
      } else {
        items.push(out.join(" "));
      }
    }
    items.join(", ")
  }

  fn canonicalize_animation(value: &str) -> String {
    // Canonical order per single animation:
    // name | duration | timing-function | delay | iteration-count | direction | fill-mode | play-state
    let mut items: Vec<String> = Vec::new();
    for item in comma(value) {
      let mut name: Option<String> = None;
      let mut duration: Option<String> = None;
      let mut delay: Option<String> = None;
      let mut timing: Option<String> = None;
      let mut iteration: Option<String> = None;
      let mut direction: Option<String> = None;
      let mut fill: Option<String> = None;
      let mut play: Option<String> = None;

      for t in space(&item) {
        let lower = t.to_lowercase();
        if is_time_token(&lower) {
          if duration.is_none() {
            duration = Some(t);
          } else if delay.is_none() {
            delay = Some(t);
          }
          continue;
        }
        if is_timing_function(&lower) {
          if timing.is_none() {
            timing = Some(t);
          }
          continue;
        }
        if matches!(lower.as_str(), "infinite") || is_integer(&lower) {
          if iteration.is_none() {
            iteration = Some(t);
          }
          continue;
        }
        if matches!(
          lower.as_str(),
          "normal" | "reverse" | "alternate" | "alternate-reverse"
        ) {
          if direction.is_none() {
            direction = Some(t);
          }
          continue;
        }
        if matches!(lower.as_str(), "none" | "forwards" | "backwards" | "both") {
          if fill.is_none() {
            fill = Some(t);
          }
          continue;
        }
        if matches!(lower.as_str(), "running" | "paused") {
          if play.is_none() {
            play = Some(t);
          }
          continue;
        }
        if name.is_none() {
          name = Some(t);
        }
      }
      let mut out: Vec<String> = Vec::new();
      if let Some(n) = name {
        out.push(n);
      }
      if let Some(d) = duration {
        out.push(d);
      }
      if let Some(tf) = timing {
        out.push(tf);
      }
      if let Some(dl) = delay {
        out.push(dl);
      }
      if let Some(it) = iteration {
        out.push(it);
      }
      if let Some(di) = direction {
        out.push(di);
      }
      if let Some(f) = fill {
        out.push(f);
      }
      if let Some(ps) = play {
        out.push(ps);
      }
      if out.is_empty() {
        items.push(item);
      } else {
        items.push(out.join(" "));
      }
    }
    items.join(", ")
  }

  fn canonicalize_columns(value: &str) -> String {
    // Order: <column-count> <column-width>
    let mut count: Option<String> = None;
    let mut width: Option<String> = None;
    for t in space(value) {
      let lower = t.to_lowercase();
      let is_length = lower.chars().any(|c| c.is_ascii_digit())
        && (lower.ends_with("px")
          || lower.ends_with("em")
          || lower.ends_with("rem")
          || lower.ends_with("vw")
          || lower.ends_with("vh")
          || lower.ends_with("vmin")
          || lower.ends_with("vmax")
          || lower.ends_with("cm")
          || lower.ends_with("mm")
          || lower.ends_with("in")
          || lower.ends_with("pt")
          || lower.ends_with("pc")
          || lower.ends_with("q")
          || lower.ends_with("ch")
          || lower.ends_with("ex"));
      if (is_integer(&lower) || lower == "auto") && count.is_none() {
        count = Some(t);
      } else if (is_length || lower == "auto") && width.is_none() {
        width = Some(t);
      }
    }
    let mut out: Vec<String> = Vec::new();
    if let Some(c) = count {
      out.push(c);
    }
    if let Some(w) = width {
      out.push(w);
    }
    if out.is_empty() {
      value.to_string()
    } else {
      out.join(" ")
    }
  }

  fn canonicalize_box_shadow(value: &str) -> String {
    // Normalize each shadow: [inset?] <x> <y> [blur] [spread] [color]
    let mut items: Vec<String> = Vec::new();
    for item in comma(value) {
      let mut inset: Option<String> = None;
      let mut color: Option<String> = None;
      let mut lengths: Vec<String> = Vec::new();
      for t in space(&item) {
        let lower = t.to_lowercase();
        if lower == "inset" && inset.is_none() {
          inset = Some(t);
          continue;
        }
        let is_color = lower.starts_with('#')
          || lower.starts_with("rgb(")
          || lower.starts_with("rgba(")
          || lower.starts_with("hsl(")
          || lower.starts_with("hsla(")
          || lower.starts_with("color(");
        if is_color && color.is_none() {
          color = Some(t);
          continue;
        }
        lengths.push(t);
      }
      let mut out: Vec<String> = Vec::new();
      if let Some(i) = inset {
        out.push(i);
      }
      out.extend(lengths);
      if let Some(c) = color {
        out.push(c);
      }
      if out.is_empty() {
        items.push(item);
      } else {
        items.push(out.join(" "));
      }
    }
    items.join(", ")
  }

  pc::plugin("postcss-ordered-values")
    // Box-model shorthands
    .decl_filter("margin", |decl, _| {
      let current = decl.value();
      let next = minimize_box_shorthand(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .decl_filter("padding", |decl, _| {
      let current = decl.value();
      let next = minimize_box_shorthand(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .decl_filter("border-color", |decl, _| {
      let current = decl.value();
      let next = minimize_box_shorthand(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .decl_filter("border-width", |decl, _| {
      let current = decl.value();
      let next = minimize_box_shorthand(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .decl_filter("border-style", |decl, _| {
      let current = decl.value();
      let next = minimize_box_shorthand(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    // border/outline shorthands
    .decl_filter("border", |decl, _| {
      let current = decl.value();
      let next = canonicalize_border_outline(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .decl_filter("outline", |decl, _| {
      let current = decl.value();
      let next = canonicalize_border_outline(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    // list-style and friends
    .decl_filter("list-style", |decl, _| {
      let current = decl.value();
      let next = canonicalize_list_style(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    // flex-flow
    .decl_filter("flex-flow", |decl, _| {
      let current = decl.value();
      let next = canonicalize_flex_flow(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    // transition/animation
    .decl_filter("transition", |decl, _| {
      let current = decl.value();
      let next = canonicalize_transition(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .decl_filter("animation", |decl, _| {
      let current = decl.value();
      let next = canonicalize_animation(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    // columns
    .decl_filter("columns", |decl, _| {
      let current = decl.value();
      let next = canonicalize_columns(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    // box-shadow
    .decl_filter("box-shadow", |decl, _| {
      let current = decl.value();
      let next = canonicalize_box_shadow(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
pub fn discard_comments_plugin() -> pc::BuiltPlugin {
  pc::plugin("postcss-discard-comments")
    .once(|root, _| {
      match root {
        pc::RootLike::Root(r) => {
          r.walk_comments(|c, _| {
            postcss::ast::Node::remove_self(&c);
            true
          });
        }
        pc::RootLike::Document(d) => {
          d.walk_comments(|c, _| {
            postcss::ast::Node::remove_self(&c);
            true
          });
        }
      }
      Ok(())
    })
    .build()
}

#[cfg(feature = "postcss_engine")]
pub fn normalize_current_color_plugin() -> pc::BuiltPlugin {
  // Canonicalize any case-variant of currentColor to exactly `currentColor`.
  pc::plugin("normalize-current-color")
    .decl(|decl, _| {
      let current = decl.value();
      let mut out = String::new();
      for token in postcss::list::space(&current) {
        if token.eq_ignore_ascii_case("currentcolor") {
          if !out.is_empty() {
            out.push(' ');
          }
          out.push_str("currentColor");
        } else {
          if !out.is_empty() {
            out.push(' ');
          }
          out.push_str(&token);
        }
      }
      if !out.is_empty() && out != current {
        decl.set_value(out);
      }
      Ok(())
    })
    .build()
}
