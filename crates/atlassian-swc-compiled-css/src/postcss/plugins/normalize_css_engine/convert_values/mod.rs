use crate::postcss::value_parser as vp;
use postcss as pc;

fn strip_leading_dot(item: &str) -> String {
  if item.starts_with('.') {
    item[1..].to_string()
  } else {
    item.to_string()
  }
}

fn is_length_unit(u: &str) -> bool {
  matches!(
    u,
    "em"
      | "ex"
      | "ch"
      | "rem"
      | "vw"
      | "vh"
      | "vmin"
      | "vmax"
      | "cm"
      | "mm"
      | "q"
      | "in"
      | "pt"
      | "pc"
      | "px"
  )
}

fn drop_leading_zero(number: f64) -> String {
  let s = number.to_string();
  if number.fract() != 0.0 {
    if s.starts_with('0') {
      return s.trim_start_matches('0').to_string();
    }
    if s.starts_with("-0") {
      return format!("-{}", &s[2..]);
    }
  }
  s
}

fn convert_length(number: f64, unit: &str) -> String {
  // Only a subset: px, pt, pc, in
  let factor = match unit {
    "in" => 96.0,
    "px" => 1.0,
    "pt" => 4.0 / 3.0,
    "pc" => 16.0,
    _ => return format!("{}{}", drop_leading_zero(number), unit),
  };
  let base = number * factor;
  let candidates = ["in", "px", "pt", "pc"];
  let mut best = String::new();
  for u in candidates {
    if u == unit {
      continue;
    }
    let val = base
      / match u {
        "in" => 96.0,
        "px" => 1.0,
        "pt" => 4.0 / 3.0,
        "pc" => 16.0,
        _ => 1.0,
      };
    let s = format!("{}{}", drop_leading_zero(val), u);
    if best.is_empty() {
      best = s;
    } else if best.len() < s.len() {
      // keep existing best
    } else {
      best = s;
    }
  }
  let current = format!("{}{}", drop_leading_zero(number), unit);
  if !best.is_empty() && best.len() < current.len() {
    best
  } else {
    current
  }
}

fn convert_time(number: f64, unit: &str) -> String {
  let factor = match unit {
    "s" => 1000.0,
    "ms" => 1.0,
    _ => return format!("{}{}", drop_leading_zero(number), unit),
  };
  let base = number * factor;
  let candidates = ["s", "ms"];
  let mut best = String::new();
  for u in candidates {
    if u == unit {
      continue;
    }
    let val = base
      / match u {
        "s" => 1000.0,
        "ms" => 1.0,
        _ => 1.0,
      };
    let s = format!("{}{}", drop_leading_zero(val), u);
    if best.is_empty() {
      best = s;
    } else if best.len() < s.len() {
      // keep
    } else {
      best = s;
    }
  }
  let current = format!("{}{}", drop_leading_zero(number), unit);
  if !best.is_empty() && best.len() < current.len() {
    best
  } else {
    current
  }
}

fn convert_angle(number: f64, unit: &str) -> String {
  let factor = match unit {
    "turn" => 360.0,
    "deg" => 1.0,
    _ => return format!("{}{}", drop_leading_zero(number), unit),
  };
  let base = number * factor;
  let candidates = ["turn", "deg"];
  let mut best = String::new();
  for u in candidates {
    if u == unit {
      continue;
    }
    let val = base
      / match u {
        "turn" => 360.0,
        "deg" => 1.0,
        _ => 1.0,
      };
    let s = format!("{}{}", drop_leading_zero(val), u);
    if best.is_empty() {
      best = s;
    } else if best.len() < s.len() {
      // keep
    } else {
      best = s;
    }
  }
  let current = format!("{}{}", drop_leading_zero(number), unit);
  if !best.is_empty() && best.len() < current.len() {
    best
  } else {
    current
  }
}

#[derive(Clone, Copy)]
struct ConvertValueOptions {
  length: bool,
  time: bool,
  angle: bool,
}

impl Default for ConvertValueOptions {
  fn default() -> Self {
    Self {
      length: true,
      time: true,
      angle: true,
    }
  }
}

fn convert_value(number: f64, unit: &str, opts: ConvertValueOptions) -> String {
  let mut value = format!("{}{}", drop_leading_zero(number), unit);
  let unit_lower = unit.to_ascii_lowercase();
  let mut converted: Option<String> = None;

  if opts.length {
    match unit_lower.as_str() {
      "in" | "px" | "pt" | "pc" => {
        converted = Some(convert_length(number, unit_lower.as_str()));
      }
      _ => {}
    }
  }

  if opts.time {
    match unit_lower.as_str() {
      "s" | "ms" => {
        converted = Some(convert_time(number, unit_lower.as_str()));
      }
      _ => {}
    }
  }

  if opts.angle {
    match unit_lower.as_str() {
      "turn" | "deg" => {
        converted = Some(convert_angle(number, unit_lower.as_str()));
      }
      _ => {}
    }
  }

  if let Some(candidate) = converted {
    if candidate.len() < value.len() {
      value = candidate;
    }
  }

  value
}

fn parse_word(node: &mut vp::Node, keep_zero_unit: bool, precision_px: Option<usize>) {
  if let vp::Node::Word { value } = node {
    if let Some(pair) = vp::unit::unit(value) {
      let num: f64 = pair.number.parse().unwrap_or(0.0);
      let u = strip_leading_dot(&pair.unit);
      if num == 0.0 {
        let keep = keep_zero_unit || (!is_length_unit(&u.to_lowercase()) && u != "%");
        *value = format!("{}{}", 0, if keep { u } else { String::new() });
      } else {
        let ul = u.to_lowercase();
        let mut out = convert_value(num, &u, ConvertValueOptions::default());
        if let Some(p) = precision_px {
          if ul == "px" && pair.number.contains('.') {
            let prec = 10f64.powi(p as i32);
            let rounded =
              (out.trim_end_matches("px").parse::<f64>().unwrap_or(num) * prec).round() / prec;
            out = format!("{}px", rounded);
          }
        }
        *value = out;
      }
    }
  }
}

fn clamp_opacity(node: &mut vp::Node) {
  if let vp::Node::Word { value } = node {
    if let Some(pair) = vp::unit::unit(value) {
      let num: f64 = pair.number.parse().unwrap_or(0.0);
      if num > 1.0 {
        *value = if pair.unit == "%" {
          format!("{}%", num)
        } else {
          format!("{}", 1)
        };
      } else if num < 0.0 {
        *value = format!("{}{}", 0, pair.unit);
      }
    }
  }
}

fn walk_nodes(nodes: &mut [vp::Node], prop: &str, keep_zero_unit: bool) {
  for node in nodes.iter_mut() {
    match node {
      vp::Node::Word { .. } => {
        parse_word(node, keep_zero_unit, None);
        if prop == "opacity" || prop == "shape-image-threshold" {
          clamp_opacity(node);
        }
      }
      vp::Node::Function {
        value,
        nodes: inner,
        ..
      } => {
        let low = value.to_lowercase();
        if low == "url" {
          continue;
        }
        let inner_keep_zero = matches!(
          low.as_str(),
          "calc" | "min" | "max" | "clamp" | "hsl" | "hsla"
        );
        walk_nodes(
          inner,
          prop,
          if inner_keep_zero {
            true
          } else {
            keep_zero_unit
          },
        );
      }
      _ => {}
    }
  }
}

fn should_keep_zero_unit(
  prop: &str,
  value_has_percent: bool,
  in_keyframes: bool,
  parent_prop: Option<&str>,
  browsers: &[&str],
) -> bool {
  // legacy IE11 percent keep for certain props
  let keep_zero_percent_props = ["max-height", "height", "min-width"];
  if value_has_percent && keep_zero_percent_props.contains(&prop) && browsers.contains(&"ie 11") {
    return true;
  }
  if in_keyframes && parent_prop == Some("stroke-dasharray") {
    return true;
  }
  let keep_when_zero = ["stroke-dashoffset", "stroke-width", "line-height"];
  keep_when_zero.contains(&prop)
}

pub fn plugin() -> pc::BuiltPlugin {
  // Browserslist wiring is omitted; default to no legacy quirks except those detectable by context.
  pc::plugin("postcss-convert-values")
    .decl(|decl, _| {
      let prop = decl.prop().to_lowercase();
      if prop.contains("flex")
        || prop.starts_with("--")
        || matches!(
          prop.as_str(),
          "descent-override"
            | "ascent-override"
            | "font-stretch"
            | "size-adjust"
            | "line-gap-override"
        )
      {
        return Ok(());
      }
      let mut parsed = vp::parse(&decl.value());
      let in_keyframes = false; // TODO: compute from context if needed
      let mut nodes = parsed.nodes.clone();
      let browsers: [&str; 0] = [];
      let keep_zero_unit = should_keep_zero_unit(
        &prop,
        decl.value().contains('%'),
        in_keyframes,
        Some(&prop),
        &browsers,
      );
      walk_nodes(&mut nodes[..], &prop, keep_zero_unit);
      parsed.nodes = nodes;
      decl.set_value(vp::stringify(&parsed.nodes));
      Ok(())
    })
    .build()
}

#[cfg(test)]
mod tests {
  use super::*;
  use postcss as pc;
  use pretty_assertions::assert_eq;

  #[test]
  fn converts_var_fallback_units_inside_calc() {
    let processor = pc::postcss_with_plugins(vec![plugin()]);
    let mut result = processor
      .process(
        "a{width:calc(100% - var(--ds-space-150, 12px));height:calc(100% - var(--ds-space-200, 16px))}",
      )
      .expect("process should succeed");

    let css = result.css().expect("css string").to_string();
    assert_eq!(
      css,
      "a{width:calc(100% - var(--ds-space-150, 9pt));height:calc(100% - var(--ds-space-200, 1pc))}"
    );
  }

  #[test]
  fn preserves_negative_percentage_unit() {
    let processor = pc::postcss_with_plugins(vec![plugin()]);
    let mut result = processor
      .process("a{background-position:-100%}")
      .expect("process should succeed");

    let css = result.css().expect("css string").to_string();
    assert!(
      css.contains("-100%"),
      "Expected -100%% to be preserved but got: {}",
      css
    );
  }
}
