use crate::hash::hash;
use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use swc_core::ecma::ast::Expr;

const INCREASE_SPECIFICITY_SELECTOR: &str = ":not(#\\#)";

#[derive(Debug, Clone)]
pub struct NormalizedCssValue {
  pub hash_value: String,
  pub output_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssOptions {
  #[serde(default)]
  pub class_hash_prefix: Option<String>,
  #[serde(default)]
  pub class_name_compression_map: BTreeMap<String, String>,
  #[serde(default)]
  pub increase_specificity: bool,
  #[serde(default)]
  pub sort_at_rules: bool,
  #[serde(default)]
  pub sort_shorthand: bool,
  #[serde(default)]
  pub flatten_multiple_selectors: bool,
}

impl Default for CssOptions {
  fn default() -> Self {
    Self {
      class_hash_prefix: None,
      class_name_compression_map: BTreeMap::new(),
      increase_specificity: false,
      sort_at_rules: true,
      sort_shorthand: true,
      flatten_multiple_selectors: false,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomicRule {
  pub class_name: String,
  pub css: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeCssVariable {
  pub name: String,
  pub expression: Expr,
  pub prefix: Option<String>,
  pub suffix: Option<String>,
  pub allow_static_substitution: bool,
}

impl RuntimeCssVariable {
  pub fn new(
    name: String,
    expression: Expr,
    prefix: Option<String>,
    suffix: Option<String>,
  ) -> Self {
    Self {
      name,
      expression,
      prefix,
      suffix,
      allow_static_substitution: true,
    }
  }
}

#[derive(Debug, Clone)]
pub struct CssArtifacts {
  pub rules: Vec<AtomicRule>,
  pub raw_rules: Vec<String>,
  pub runtime_variables: Vec<RuntimeCssVariable>,
  pub runtime_class_conditions: Vec<RuntimeClassCondition>,
}

impl CssArtifacts {
  pub fn push(&mut self, rule: AtomicRule) {
    self.rules.push(rule);
  }

  pub fn push_raw(&mut self, css: String) {
    self.raw_rules.push(css);
  }

  pub fn merge(&mut self, other: CssArtifacts) {
    self.rules.extend(other.rules);
    self.raw_rules.extend(other.raw_rules);
    self.runtime_variables.extend(other.runtime_variables);
    self
      .runtime_class_conditions
      .extend(other.runtime_class_conditions);
  }

  pub fn push_variable(&mut self, variable: RuntimeCssVariable) {
    self.runtime_variables.push(variable);
  }

  pub fn push_class_condition(&mut self, condition: RuntimeClassCondition) {
    self.runtime_class_conditions.push(condition);
  }

  pub fn class_names(&self) -> impl Iterator<Item = &str> {
    self.rules.iter().map(|rule| rule.class_name.as_str())
  }
}

impl Default for CssArtifacts {
  fn default() -> Self {
    Self {
      rules: Vec::new(),
      raw_rules: Vec::new(),
      runtime_variables: Vec::new(),
      runtime_class_conditions: Vec::new(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeClassCondition {
  pub test: Expr,
  pub when_true: Vec<String>,
  pub when_false: Vec<String>,
}

impl RuntimeClassCondition {
  pub fn new(test: Expr, when_true: Vec<String>, when_false: Vec<String>) -> Self {
    Self {
      test,
      when_true,
      when_false,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AtRuleInput {
  pub name: String,
  pub params: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CssRuleInput {
  pub selectors: Vec<String>,
  pub at_rules: Vec<AtRuleInput>,
  pub property: String,
  pub value: String,
  pub raw_value: String,
  pub important: bool,
}

fn normalize_pseudo_element_colons(selector: &str) -> Cow<'_, str> {
  if !selector.contains("::") {
    return Cow::Borrowed(selector);
  }

  let mut output = String::with_capacity(selector.len());
  let mut chars = selector.chars().peekable();
  let mut in_single_quote = false;
  let mut in_double_quote = false;
  let mut escape_next = false;

  while let Some(ch) = chars.next() {
    if escape_next {
      output.push(ch);
      escape_next = false;
      continue;
    }

    match ch {
      '\\' => {
        escape_next = true;
        output.push(ch);
      }
      '\'' if !in_double_quote => {
        in_single_quote = !in_single_quote;
        output.push(ch);
      }
      '"' if !in_single_quote => {
        in_double_quote = !in_double_quote;
        output.push(ch);
      }
      ':' if !in_single_quote && !in_double_quote => {
        output.push(':');
        if matches!(chars.peek(), Some(':')) {
          chars.next();
        }
      }
      _ => output.push(ch),
    }
  }

  Cow::Owned(output)
}

fn ensure_double_colon_placeholder(selector: &str) -> Cow<'_, str> {
  if !selector.contains(":placeholder") {
    return Cow::Borrowed(selector);
  }

  let mut output = String::with_capacity(selector.len());
  let bytes = selector.as_bytes();
  let mut index = 0usize;

  while index < bytes.len() {
    if bytes[index] == b':' {
      let remaining = &selector[index..];
      if remaining.starts_with("::placeholder") {
        output.push_str("::placeholder");
        index += "::placeholder".len();
        continue;
      }
      if remaining.starts_with(":placeholder") {
        output.push_str("::placeholder");
        index += ":placeholder".len();
        continue;
      }
    }
    output.push(bytes[index] as char);
    index += 1;
  }

  Cow::Owned(output)
}

pub fn normalize_selector(selector: Option<&str>) -> String {
  match selector {
    None => "&".to_string(),
    Some(raw) => {
      let trimmed = raw.trim();
      let pseudo_normalized = normalize_pseudo_element_colons(trimmed);
      let normalized_minified =
        normalize_attribute_selector_quotes(&minify_selector(pseudo_normalized.as_ref()));
      let normalized = ensure_double_colon_placeholder(&normalized_minified).into_owned();
      if normalized.contains('&') {
        if normalized.starts_with(':') {
          return format!("&{}", normalized);
        }
        let mut rebuilt = String::with_capacity(normalized.len());
        let mut segments = normalized.split('&');
        let raw_segments: Vec<&str> = raw.split('&').collect();
        let mut raw_segments_iter = raw_segments.iter();
        let raw_contains_ampersand = raw.contains('&');
        if let Some(first) = segments.next() {
          rebuilt.push_str(first);
          let _ = raw_segments_iter.next();
        }
        for segment in segments {
          rebuilt.push('&');
          let raw_segment = raw_segments_iter.next().copied().unwrap_or(segment);
          let had_leading_space = raw_segment.starts_with(char::is_whitespace);
          let trimmed = segment.trim_start();
          if trimmed.is_empty() {
            continue;
          }
          match trimmed.chars().next() {
            Some('>' | '+' | '~') => {
              let mut chars = trimmed.chars();
              let combinator = chars.next().unwrap();
              let rest_raw = chars.as_str();
              let rest_trimmed = rest_raw.trim_start();
              let rest: Cow<'_, str> = if let Some(after_star) = rest_trimmed.strip_prefix("*::") {
                Cow::Owned(format!("::{}", after_star))
              } else if let Some(after_star) = rest_trimmed.strip_prefix("*:") {
                Cow::Owned(format!(":{}", after_star))
              } else {
                Cow::Borrowed(rest_trimmed)
              };
              let rest_str = rest.as_ref();
              if had_leading_space && !rebuilt.ends_with(' ') && !raw_contains_ampersand {
                rebuilt.push(' ');
              }
              rebuilt.push(combinator);
              rebuilt.push_str(rest_str);
            }
            Some(':') | Some('[') => {
              if had_leading_space && !rebuilt.ends_with(' ') {
                rebuilt.push(' ');
              }
              rebuilt.push_str(trimmed);
            }
            _ => {
              rebuilt.push(' ');
              rebuilt.push_str(trimmed);
            }
          }
        }
        return rebuilt;
      } else if normalized.is_empty() {
        "&".to_string()
      } else if normalized.starts_with(':') || normalized.starts_with('[') {
        format!("&{}", normalized)
      } else {
        match normalized.chars().next() {
          Some('>' | '+' | '~') => {
            let mut chars = normalized.chars();
            let combinator = chars.next().unwrap();
            let rest_raw = chars.as_str();
            let rest_trimmed = rest_raw.trim_start();
            let rest: Cow<'_, str> = if let Some(after_star) = rest_trimmed.strip_prefix("*::") {
              Cow::Owned(format!("::{}", after_star))
            } else if let Some(after_star) = rest_trimmed.strip_prefix("*:") {
              Cow::Owned(format!(":{}", after_star))
            } else {
              Cow::Borrowed(rest_trimmed)
            };
            let rest_str = rest.as_ref();
            let needs_space = matches!(rest_str.chars().next(), Some(':' | '[' | '*'));
            if needs_space {
              format!("& {}{}", combinator, rest_str)
            } else {
              format!("&{}{}", combinator, rest_str)
            }
          }
          _ => format!("& {}", normalized),
        }
      }
    }
  }
}

fn normalize_attribute_selector_quotes(selector: &str) -> String {
  let mut interim = String::with_capacity(selector.len());
  let mut in_attribute = false;
  let mut current_quote: Option<char> = None;

  for ch in selector.chars() {
    match ch {
      '[' if current_quote.is_none() => {
        in_attribute = true;
        interim.push(ch);
      }
      ']' if current_quote.is_none() => {
        in_attribute = false;
        interim.push(ch);
      }
      '\'' if in_attribute => {
        if current_quote.is_none() {
          current_quote = Some('"');
        } else {
          current_quote = None;
        }
        interim.push('"');
      }
      '"' if in_attribute => {
        if current_quote.is_none() {
          current_quote = Some('"');
        } else {
          current_quote = None;
        }
        interim.push('"');
      }
      _ => {
        interim.push(ch);
      }
    }
  }

  let mut result = String::with_capacity(interim.len());
  let mut chars = interim.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '=' && matches!(chars.peek(), Some('"')) {
      result.push('=');
      chars.next();
      let mut value = String::new();
      while let Some(&next) = chars.peek() {
        chars.next();
        if next == '"' {
          break;
        }
        value.push(next);
      }
      if !value.is_empty()
        && value
          .chars()
          .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '\\')
      {
        result.push_str(&value);
      } else {
        result.push('"');
        result.push_str(&value);
        result.push('"');
      }
      continue;
    }
    result.push(ch);
  }

  result
}

fn named_color_hex(value: &str) -> Option<&'static str> {
  match value {
    "aliceblue" => Some("#f0f8ff"),
    "antiquewhite" => Some("#faebd7"),
    "aqua" => Some("#00ffff"),
    "aquamarine" => Some("#7fffd4"),
    "azure" => Some("#f0ffff"),
    "beige" => Some("#f5f5dc"),
    "bisque" => Some("#ffe4c4"),
    "black" => Some("#000000"),
    "blanchedalmond" => Some("#ffebcd"),
    "blue" => Some("#0000ff"),
    "blueviolet" => Some("#8a2be2"),
    "brown" => Some("#a52a2a"),
    "burlywood" => Some("#deb887"),
    "cadetblue" => Some("#5f9ea0"),
    "chartreuse" => Some("#7fff00"),
    "chocolate" => Some("#d2691e"),
    "coral" => Some("#ff7f50"),
    "cornflowerblue" => Some("#6495ed"),
    "cornsilk" => Some("#fff8dc"),
    "crimson" => Some("#dc143c"),
    "darkblue" => Some("#00008b"),
    "darkcyan" => Some("#008b8b"),
    "darkgoldenrod" => Some("#b8860b"),
    "darkgray" => Some("#a9a9a9"),
    "darkgreen" => Some("#006400"),
    "darkkhaki" => Some("#bdb76b"),
    "darkmagenta" => Some("#8b008b"),
    "darkolivegreen" => Some("#556b2f"),
    "darkorange" => Some("#ff8c00"),
    "darkorchid" => Some("#9932cc"),
    "darkred" => Some("#8b0000"),
    "darksalmon" => Some("#e9967a"),
    "darkseagreen" => Some("#8fbc8f"),
    "darkslateblue" => Some("#483d8b"),
    "darkslategray" => Some("#2f4f4f"),
    "darkturquoise" => Some("#00ced1"),
    "darkviolet" => Some("#9400d3"),
    "deeppink" => Some("#ff1493"),
    "deepskyblue" => Some("#00bfff"),
    "dimgray" => Some("#696969"),
    "dodgerblue" => Some("#1e90ff"),
    "firebrick" => Some("#b22222"),
    "floralwhite" => Some("#fffaf0"),
    "forestgreen" => Some("#228b22"),
    "fuchsia" => Some("#ff00ff"),
    "gainsboro" => Some("#dcdcdc"),
    "ghostwhite" => Some("#f8f8ff"),
    "gold" => Some("#ffd700"),
    "goldenrod" => Some("#daa520"),
    "gray" => Some("#808080"),
    "green" => Some("#008000"),
    "greenyellow" => Some("#adff2f"),
    "honeydew" => Some("#f0fff0"),
    "hotpink" => Some("#ff69b4"),
    "indianred" => Some("#cd5c5c"),
    "indigo" => Some("#4b0082"),
    "ivory" => Some("#fffff0"),
    "khaki" => Some("#f0e68c"),
    "lavender" => Some("#e6e6fa"),
    "lavenderblush" => Some("#fff0f5"),
    "lawngreen" => Some("#7cfc00"),
    "lemonchiffon" => Some("#fffacd"),
    "lightblue" => Some("#add8e6"),
    "lightcoral" => Some("#f08080"),
    "lightcyan" => Some("#e0ffff"),
    "lightgoldenrodyellow" => Some("#fafad2"),
    "lightgray" => Some("#d3d3d3"),
    "lightgreen" => Some("#90ee90"),
    "lightpink" => Some("#ffb6c1"),
    "lightsalmon" => Some("#ffa07a"),
    "lightseagreen" => Some("#20b2aa"),
    "lightskyblue" => Some("#87cefa"),
    "lightslategray" => Some("#778899"),
    "lightsteelblue" => Some("#b0c4de"),
    "lightyellow" => Some("#ffffe0"),
    "lime" => Some("#00ff00"),
    "limegreen" => Some("#32cd32"),
    "linen" => Some("#faf0e6"),
    "maroon" => Some("#800000"),
    "mediumaquamarine" => Some("#66cdaa"),
    "mediumblue" => Some("#0000cd"),
    "mediumorchid" => Some("#ba55d3"),
    "mediumpurple" => Some("#9370db"),
    "mediumseagreen" => Some("#3cb371"),
    "mediumslateblue" => Some("#7b68ee"),
    "mediumspringgreen" => Some("#00fa9a"),
    "mediumturquoise" => Some("#48d1cc"),
    "mediumvioletred" => Some("#c71585"),
    "midnightblue" => Some("#191970"),
    "mintcream" => Some("#f5fffa"),
    "mistyrose" => Some("#ffe4e1"),
    "moccasin" => Some("#ffe4b5"),
    "navajowhite" => Some("#ffdead"),
    "navy" => Some("#000080"),
    "oldlace" => Some("#fdf5e6"),
    "olive" => Some("#808000"),
    "olivedrab" => Some("#6b8e23"),
    "orange" => Some("#ff8000"),
    "orangered" => Some("#ff4500"),
    "orchid" => Some("#da70d6"),
    "palegoldenrod" => Some("#eee8aa"),
    "palegreen" => Some("#98fb98"),
    "paleturquoise" => Some("#afeeee"),
    "palevioletred" => Some("#db7093"),
    "papayawhip" => Some("#ffefd5"),
    "peachpuff" => Some("#ffdab9"),
    "peru" => Some("#cd853f"),
    "pink" => Some("#ffc0cb"),
    "plum" => Some("#dda0dd"),
    "powderblue" => Some("#b0e0e6"),
    "purple" => Some("#800080"),
    "rebeccapurple" => Some("#663399"),
    "red" => Some("#ff0000"),
    "rosybrown" => Some("#bc8f8f"),
    "royalblue" => Some("#4169e1"),
    "saddlebrown" => Some("#8b4513"),
    "salmon" => Some("#fa8072"),
    "sandybrown" => Some("#f4a460"),
    "seagreen" => Some("#2e8b57"),
    "seashell" => Some("#fff5ee"),
    "sienna" => Some("#a0522d"),
    "silver" => Some("#c0c0c0"),
    "skyblue" => Some("#87ceeb"),
    "slateblue" => Some("#6a5acd"),
    "slategray" => Some("#708090"),
    "snow" => Some("#fffafa"),
    "springgreen" => Some("#00ff7f"),
    "steelblue" => Some("#4682b4"),
    "tan" => Some("#d2b48c"),
    "teal" => Some("#008080"),
    "thistle" => Some("#d8bfd8"),
    "tomato" => Some("#ff6347"),
    "turquoise" => Some("#40e0d0"),
    "violet" => Some("#ee82ee"),
    "wheat" => Some("#f5deb3"),
    "white" => Some("#ffffff"),
    "whitesmoke" => Some("#f5f5f5"),
    "yellow" => Some("#ffff00"),
    "yellowgreen" => Some("#9acd32"),
    _ => None,
  }
}

pub fn add_unit_if_needed(name: &str, value: &str) -> String {
  if value.is_empty() {
    return String::new();
  }

  if let Ok(number) = value.parse::<f64>() {
    let unitless = if is_unitless_property(name) {
      true
    } else if let Some(camel) = crate::to_camel_case(name) {
      is_unitless_property(&camel)
    } else {
      false
    };

    if number == 0.0 || unitless {
      return trim_numeric(value);
    }

    return format!("{}px", trim_numeric(value));
  }

  trim_numeric(value)
}

fn lowercase_hex_literals(input: &str) -> String {
  let mut bytes = input.as_bytes().to_vec();
  let mut index = 0usize;
  while index < bytes.len() {
    if bytes[index] == b'#' {
      let mut inner = index + 1;
      while inner < bytes.len() {
        let ch = bytes[inner];
        if (ch as char).is_ascii_hexdigit() {
          bytes[inner] = ch.to_ascii_lowercase();
          inner += 1;
        } else {
          break;
        }
      }
      index = inner;
    } else {
      index += 1;
    }
  }

  String::from_utf8(bytes).expect("css value should be valid utf8")
}

fn shorten_hex_literals(input: &str) -> String {
  let mut output = String::with_capacity(input.len());
  let chars: Vec<char> = input.chars().collect();
  let mut index = 0usize;

  while index < chars.len() {
    let ch = chars[index];
    if ch == '#' {
      if index + 8 < chars.len() {
        let slice = &chars[index + 1..index + 9];
        if slice.iter().all(|c| c.is_ascii_hexdigit()) {
          if slice[0] == slice[1]
            && slice[2] == slice[3]
            && slice[4] == slice[5]
            && slice[6] == slice[7]
          {
            output.push('#');
            output.push(slice[0]);
            output.push(slice[2]);
            output.push(slice[4]);
            output.push(slice[6]);
            index += 9;
            continue;
          }
        }
      }
      if index + 6 < chars.len()
        && !(index + 7 < chars.len() && chars[index + 7].is_ascii_hexdigit())
      {
        let slice = &chars[index + 1..index + 7];
        if slice.iter().all(|c| c.is_ascii_hexdigit())
          && slice[0] == slice[1]
          && slice[2] == slice[3]
          && slice[4] == slice[5]
        {
          output.push('#');
          output.push(slice[0]);
          output.push(slice[2]);
          output.push(slice[4]);
          index += 7;
          continue;
        }
      }
    }

    output.push(ch);
    index += 1;
  }

  output
}

fn minify_whitespace(value: &str) -> String {
  let mut output = String::with_capacity(value.len());
  let mut chars = value.chars().peekable();
  let mut last_was_space = false;

  while let Some(ch) = chars.next() {
    if ch.is_ascii_whitespace() {
      if !last_was_space {
        output.push(' ');
        last_was_space = true;
      }
      continue;
    }

    if ch == ',' {
      if output.ends_with(' ') {
        output.pop();
      }
      output.push(ch);
      while matches!(chars.peek(), Some(next) if next.is_ascii_whitespace()) {
        chars.next();
      }
      last_was_space = false;
      continue;
    }

    if ch == '/' {
      if output.ends_with(' ') {
        output.pop();
      }
      output.push(ch);
      while matches!(chars.peek(), Some(next) if next.is_ascii_whitespace()) {
        chars.next();
      }
      last_was_space = false;
      continue;
    }

    last_was_space = false;
    output.push(ch);
  }

  if output.ends_with(' ') {
    output.pop();
  }

  output
}

fn starts_with_ignore_ascii(bytes: &[u8], needle: &[u8]) -> bool {
  if bytes.len() < needle.len() {
    return false;
  }
  bytes[..needle.len()]
    .iter()
    .zip(needle.iter())
    .all(|(byte, expected)| byte.to_ascii_lowercase() == expected.to_ascii_lowercase())
}

fn ensure_var_fallback_space(value: &str) -> String {
  let mut output = String::with_capacity(value.len());
  let mut index = 0usize;
  let mut changed = false;

  while let Some(rel_start) = value[index..].find("var(") {
    let start = index + rel_start;
    output.push_str(&value[index..start]);
    let substring = &value[start..];
    let mut iter = substring.char_indices();
    let mut depth = 0i32;
    let mut comma_rel: Option<usize> = None;
    let mut end_rel: Option<usize> = None;

    while let Some((offset, ch)) = iter.next() {
      match ch {
        '(' => {
          depth += 1;
        }
        ')' => {
          depth -= 1;
          if depth == 0 {
            end_rel = Some(offset + ch.len_utf8());
            break;
          }
        }
        ',' => {
          if depth == 1 && comma_rel.is_none() {
            comma_rel = Some(offset);
          }
        }
        _ => {}
      }
    }

    let Some(end_rel) = end_rel else {
      output.push_str(substring);
      return output;
    };

    if let Some(comma_rel) = comma_rel {
      let after_comma_rel = {
        let mut pos = comma_rel + 1;
        while pos < end_rel {
          let ch = substring[pos..].chars().next().unwrap();
          if ch.is_whitespace() {
            pos += ch.len_utf8();
          } else {
            break;
          }
        }
        pos
      };

      if after_comma_rel < end_rel - 1 {
        output.push_str(&substring[..comma_rel + 1]);
        output.push(' ');
        output.push_str(&substring[after_comma_rel..end_rel]);
        changed = true;
      } else {
        output.push_str(&substring[..end_rel]);
      }
    } else {
      output.push_str(&substring[..end_rel]);
    }

    index = start + end_rel;
  }

  output.push_str(&value[index..]);
  if changed { output } else { value.to_string() }
}

pub fn normalize_css_value(value: &str) -> NormalizedCssValue {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return NormalizedCssValue {
      hash_value: String::new(),
      output_value: String::new(),
    };
  }

  let mut semantic = trimmed.to_string();
  let lower_trimmed = trimmed.to_ascii_lowercase();
  if let Some(hex) = named_color_hex(&lower_trimmed) {
    let shortened = shorten_hex_literals(hex);
    let candidate = if shortened.len() < hex.len() {
      shortened
    } else {
      hex.to_string()
    };
    if candidate.len() < semantic.len() {
      semantic = candidate;
    } else if hex.len() < semantic.len() {
      semantic = hex.to_string();
    }
  }
  if lower_trimmed == "currentcolor" || lower_trimmed == "current-color" {
    semantic = "currentColor".to_string();
  }
  semantic = lowercase_hex_literals(&semantic);
  semantic = shorten_hex_literals(&semantic);
  semantic = strip_decimal_leading_zeros(&semantic);
  semantic = strip_trailing_decimal_zeros(&semantic);
  semantic = convert_length_units(&semantic);
  semantic = convert_time_units(&semantic);
  semantic = convert_color_functions_to_hex(&semantic);
  semantic = lowercase_hex_literals(&semantic);
  semantic = shorten_hex_literals(&semantic);
  semantic = strip_zero_units(&semantic);
  semantic = normalize_calc_multiplication(&semantic);
  semantic = normalize_calc_subtraction(&semantic);
  semantic = convert_rotate_deg_to_turn(&semantic);
  semantic = restore_calc_zero_fallbacks(&semantic);
  let hash_value = ensure_var_fallback_space(&semantic);
  let output = minify_whitespace(&semantic);

  NormalizedCssValue {
    hash_value,
    output_value: output,
  }
}

fn convert_pc_to_px(value: &str) -> Option<String> {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return None;
  }
  let (core, important) = if let Some(stripped) = trimmed.strip_suffix("!important") {
    (stripped.trim_end(), Some("!important"))
  } else {
    (trimmed, None)
  };
  let mut chars = core.chars();
  let mut sign = "";
  if let Some(first) = chars.next() {
    if first == '-' || first == '+' {
      sign = if first == '-' { "-" } else { "" };
    } else {
      chars = core.chars();
    }
  }
  let number_part = chars.as_str().strip_suffix("pc")?.trim();
  if number_part.is_empty() {
    return None;
  }
  let parsed = number_part.parse::<i64>().ok()?;
  let px_value = parsed.checked_mul(16)?;
  let mut result = String::new();
  if sign == "-" && px_value > 0 {
    result.push('-');
  }
  result.push_str(px_value.abs().to_string().as_str());
  result.push_str("px");
  if let Some(suffix) = important {
    result.push(' ');
    result.push_str(suffix);
  }
  Some(result)
}

fn is_time_token(token: &str) -> bool {
  let trimmed = token.trim_end_matches("!important");
  let lower = trimmed.trim().to_ascii_lowercase();
  if let Some(stripped) = lower.strip_suffix('s') {
    if stripped.ends_with('m') {
      return stripped[..stripped.len() - 1].parse::<f64>().is_ok();
    }
    return stripped.parse::<f64>().is_ok();
  }
  false
}

fn is_timing_function(token: &str) -> bool {
  let lower = token.to_ascii_lowercase();
  matches!(
    lower.as_str(),
    "ease" | "linear" | "ease-in" | "ease-out" | "ease-in-out" | "step-start" | "step-end"
  ) || lower.starts_with("cubic-bezier(")
    || lower.starts_with("steps(")
}

struct NormalizedTransitionValue {
  hash_value: String,
  output_value: String,
}

fn split_transition_segments(value: &str) -> Vec<String> {
  let mut segments = Vec::new();
  let mut current = String::new();
  let mut paren_depth = 0usize;
  let mut in_single_quote = false;
  let mut in_double_quote = false;
  let mut escape_next = false;

  for ch in value.chars() {
    if escape_next {
      current.push(ch);
      escape_next = false;
      continue;
    }
    match ch {
      '\\' => {
        current.push(ch);
        escape_next = true;
      }
      '\'' => {
        current.push(ch);
        if !in_double_quote {
          in_single_quote = !in_single_quote;
        }
      }
      '"' => {
        current.push(ch);
        if !in_single_quote {
          in_double_quote = !in_double_quote;
        }
      }
      '(' if !in_single_quote && !in_double_quote => {
        paren_depth += 1;
        current.push(ch);
      }
      ')' if !in_single_quote && !in_double_quote => {
        if paren_depth > 0 {
          paren_depth -= 1;
        }
        current.push(ch);
      }
      ',' if paren_depth == 0 && !in_single_quote && !in_double_quote => {
        if !current.trim().is_empty() {
          segments.push(current.trim().to_string());
        }
        current.clear();
      }
      _ => current.push(ch),
    }
  }

  if !current.trim().is_empty() {
    segments.push(current.trim().to_string());
  }

  segments
}

fn normalize_transition_value(value: &str) -> NormalizedTransitionValue {
  let mut hash_segments = Vec::new();
  let mut output_segments = Vec::new();

  for segment in split_transition_segments(value) {
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    if tokens.is_empty() {
      continue;
    }

    let mut property_tokens = Vec::new();
    let mut time_tokens = Vec::new();
    let mut timing_tokens = Vec::new();
    let mut other_tokens = Vec::new();

    for token in tokens {
      if is_time_token(token) {
        time_tokens.push(token);
      } else if is_timing_function(token) {
        timing_tokens.push(token);
      } else {
        if property_tokens.is_empty() {
          property_tokens.push(token);
        } else {
          other_tokens.push(token);
        }
      }
    }

    let mut ordered = Vec::new();
    ordered.extend(property_tokens);
    ordered.extend(time_tokens);
    ordered.extend(timing_tokens);
    ordered.extend(other_tokens);

    let output_segment = ordered.join(" ");
    let hash_segment = output_segment.clone();

    output_segments.push(output_segment);
    hash_segments.push(hash_segment);
  }

  let hash_value = hash_segments.join(",");
  if std::env::var_os("COMPILED_DEBUG_HASH").is_some() {
    eprintln!(
      "[compiled-debug] transition-hash segments={:?}",
      hash_segments
    );
  }
  NormalizedTransitionValue {
    hash_value,
    output_value: output_segments.join(","),
  }
}

fn normalize_background_position(value: &str) -> Option<String> {
  let mut changed = false;
  let mut segments = Vec::new();

  for segment in value.split(',') {
    let mut tokens = Vec::new();
    for token in segment.split_whitespace() {
      let replacement = match token {
        "center" => {
          changed = true;
          "50%"
        }
        "left" | "top" => {
          changed = true;
          "0"
        }
        "right" | "bottom" => {
          changed = true;
          "100%"
        }
        _ => token,
      };
      tokens.push(replacement);
    }
    segments.push(tokens.join(" "));
  }

  if changed {
    Some(segments.join(","))
  } else {
    None
  }
}

#[allow(unused_assignments)]
fn normalize_calc_multiplication(value: &str) -> String {
  let trimmed = value.trim();
  if !trimmed.starts_with("calc(") || !trimmed.ends_with(')') {
    return value.to_string();
  }
  let inner = &trimmed[5..trimmed.len() - 1];
  let collapsed = inner.replace(' ', "");
  if collapsed.starts_with("-1*var(") {
    let bytes = inner.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
      index += 1;
    }
    if index >= bytes.len() || bytes[index] != b'-' {
      return value.to_string();
    }
    index += 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
      index += 1;
    }
    if index >= bytes.len() || bytes[index] != b'1' {
      return value.to_string();
    }
    index += 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
      index += 1;
    }
    if index >= bytes.len() || bytes[index] != b'*' {
      return value.to_string();
    }
    index += 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
      index += 1;
    }
    if index >= bytes.len() || !inner[index..].starts_with("var(") {
      return value.to_string();
    }
    let var_start = index;
    let mut depth = 0usize;
    let mut cursor = index;
    let mut var_end: Option<usize> = None;
    while cursor < bytes.len() {
      let ch = inner[cursor..]
        .chars()
        .next()
        .expect("cursor is valid character boundary");
      let ch_len = ch.len_utf8();
      match ch {
        '(' => {
          depth += 1;
          cursor += ch_len;
        }
        ')' => {
          if depth == 0 {
            var_end = Some(cursor);
            cursor += ch_len;
            break;
          } else {
            depth -= 1;
            cursor += ch_len;
          }
        }
        _ => {
          cursor += ch_len;
        }
      }
    }
    if let Some(end_index) = var_end {
      let remainder: String = inner[end_index + 1..]
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect();
      if remainder.is_empty() || remainder == "*-1" {
        let segment = inner[var_start..=end_index].trim();
        return format!("calc({segment}*-1)");
      }
    }
    let mut depth = 0usize;
    let mut end_index = None;
    for (index, ch) in collapsed.char_indices() {
      if ch == '(' {
        depth += 1;
      } else if ch == ')' {
        depth -= 1;
        if depth == 0 {
          end_index = Some(index);
          break;
        }
      }
    }
    if let Some(end) = end_index {
      let var_part = &collapsed[0..=end];
      let remainder = collapsed[end + 1..].to_string();
      if remainder.is_empty() || remainder == "*-1" {
        return format!("calc({}*-1)", var_part.trim_start_matches("-1*"));
      }
    }
  }
  value.to_string()
}

fn normalize_calc_subtraction(value: &str) -> String {
  let trimmed = value.trim();
  if !trimmed.starts_with("calc(") || !trimmed.ends_with(')') {
    return value.to_string();
  }

  let inner = &trimmed[5..trimmed.len() - 1];
  let mut result = String::with_capacity(inner.len());
  let mut index = 0usize;
  let inner_bytes = inner.as_bytes();

  while index < inner_bytes.len() {
    let remainder = &inner[index..];
    if remainder.starts_with('-') {
      let mut lookahead = index + 1;
      while lookahead < inner_bytes.len() && inner_bytes[lookahead].is_ascii_whitespace() {
        lookahead += 1;
      }
      if lookahead < inner_bytes.len() && inner_bytes[lookahead] == b'(' {
        if let Some((group, consumed)) = extract_parenthesized(inner, lookahead) {
          if let Some(terms) = split_top_level_plus(group) {
            if !result.is_empty() && !result.ends_with(' ') {
              result.push(' ');
            }
            result.push_str("- ");
            let mut term_iter = terms.iter();
            if let Some(first) = term_iter.next() {
              result.push_str(first.trim());
            }
            for term in term_iter {
              result.push_str(" - ");
              result.push_str(term.trim());
            }
            index = lookahead + consumed;
            continue;
          }
        }
      }
    }

    let ch = remainder.chars().next().expect("valid utf-8");
    result.push(ch);
    index += ch.len_utf8();
  }

  format!("calc({})", result)
}

fn convert_rotate_deg_to_turn(value: &str) -> String {
  const PREFIX: &str = "rotate(";

  let len = value.len();
  let mut index = 0usize;
  let mut last_copied = 0usize;
  let mut result: Option<String> = None;

  while index < len {
    if value[index..].starts_with(PREFIX) {
      let mut cursor = index + PREFIX.len();
      let mut depth = 0usize;
      let mut close_idx: Option<usize> = None;

      while cursor < len {
        let ch = value[cursor..]
          .chars()
          .next()
          .expect("cursor is valid character boundary");
        let ch_len = ch.len_utf8();
        match ch {
          '(' => {
            depth += 1;
            cursor += ch_len;
          }
          ')' => {
            if depth == 0 {
              close_idx = Some(cursor);
              cursor += ch_len;
              break;
            } else {
              depth -= 1;
              cursor += ch_len;
            }
          }
          _ => {
            cursor += ch_len;
          }
        }
      }

      if let Some(close_pos) = close_idx {
        let inner = &value[index + PREFIX.len()..close_pos];
        let trimmed = inner.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.ends_with("deg") {
          let number_part = trimmed[..trimmed.len() - 3].trim();
          if let Ok(angle_deg) = number_part.parse::<f64>() {
            let turns = angle_deg / 360.0;
            if turns.abs() >= 1e-9 {
              let mut formatted = if (turns - turns.round()).abs() <= 1e-9 {
                format!("{:.0}", turns.round())
              } else {
                let mut value = format!("{:.6}", turns);
                while value.contains('.') && value.ends_with('0') {
                  value.pop();
                }
                if value.ends_with('.') {
                  value.pop();
                }
                value
              };
              if formatted == "-0" {
                formatted = "0".to_string();
              }

              let buffer = result.get_or_insert_with(|| String::with_capacity(value.len()));
              buffer.push_str(&value[last_copied..index]);
              buffer.push_str("rotate(");
              buffer.push_str(&formatted);
              buffer.push_str("turn)");
              last_copied = cursor;
              index = cursor;
              continue;
            }
          }
        }
      }
    }
    if let Some(ch_len) = value[index..].chars().next().map(|ch| ch.len_utf8()) {
      index += ch_len;
    } else {
      break;
    }
  }

  if let Some(mut output) = result {
    if last_copied < len {
      output.push_str(&value[last_copied..]);
    }
    output
  } else {
    value.to_string()
  }
}

fn restore_calc_zero_fallbacks(value: &str) -> String {
  let trimmed = value.trim();
  if !trimmed.starts_with("calc(") {
    return value.to_string();
  }

  let bytes = value.as_bytes();
  let mut output = String::with_capacity(value.len());
  let mut index = 0usize;

  while index < bytes.len() {
    if bytes[index..].starts_with(b"var(") {
      let mut segment = String::from("var(");
      index += 4;
      let mut depth = 0usize;
      let mut fallback_start: Option<usize> = None;
      while index < bytes.len() {
        let ch = bytes[index] as char;
        segment.push(ch);
        match ch {
          '(' => depth += 1,
          ')' => {
            if depth == 0 {
              index += 1;
              break;
            } else {
              depth -= 1;
            }
          }
          ',' if depth == 0 && fallback_start.is_none() => {
            fallback_start = Some(segment.len());
          }
          _ => {}
        }
        index += 1;
      }
      if let Some(start) = fallback_start {
        let end = segment.len().saturating_sub(1);
        if start <= end {
          let fallback_slice = &segment[start..end];
          let whitespace_len = fallback_slice
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .map(|ch| ch.len_utf8())
            .sum::<usize>();
          let value_slice = &fallback_slice[whitespace_len..];
          if value_slice.trim() == "0" {
            let mut adjusted = String::with_capacity(segment.len() + 2);
            adjusted.push_str(&segment[..start]);
            if whitespace_len > 0 {
              adjusted.push_str(&fallback_slice[..whitespace_len]);
            } else {
              adjusted.push(' ');
            }
            adjusted.push_str("0px");
            adjusted.push(')');
            segment = adjusted;
          }
        }
      }
      output.push_str(&segment);
      continue;
    }

    output.push(bytes[index] as char);
    index += 1;
  }

  output
}

fn extract_parenthesized<'a>(input: &'a str, start: usize) -> Option<(&'a str, usize)> {
  let mut depth = 0usize;
  for (offset, ch) in input[start..].char_indices() {
    match ch {
      '(' => depth += 1,
      ')' => {
        if depth == 0 {
          return None;
        }
        depth -= 1;
        if depth == 0 {
          let end = start + offset;
          let inner = &input[start + 1..end];
          let consumed = offset + ch.len_utf8();
          return Some((inner, consumed));
        }
      }
      _ => {}
    }
  }
  None
}

fn split_top_level_plus(input: &str) -> Option<Vec<String>> {
  let mut parts = Vec::new();
  let mut depth = 0usize;
  let mut start = 0usize;
  let mut saw_plus = false;

  for (offset, ch) in input.char_indices() {
    match ch {
      '(' => depth += 1,
      ')' => {
        if depth == 0 {
          return None;
        }
        depth -= 1;
      }
      '+' if depth == 0 => {
        let segment = input[start..offset].trim();
        if segment.is_empty() {
          return None;
        }
        parts.push(segment.to_string());
        start = offset + ch.len_utf8();
        saw_plus = true;
      }
      '-' if depth == 0 => {
        return None;
      }
      _ => {}
    }
  }

  if depth != 0 {
    return None;
  }

  let last = input[start..].trim();
  if last.is_empty() || !saw_plus {
    return None;
  }
  parts.push(last.to_string());

  Some(parts)
}

fn strip_decimal_leading_zeros(value: &str) -> String {
  let mut output = String::with_capacity(value.len());
  let mut chars = value.chars().peekable();
  let mut prev: Option<char> = None;

  while let Some(ch) = chars.next() {
    if ch == '0' {
      if let Some('.') = chars.peek().copied() {
        if !prev
          .map(|c| c.is_ascii_digit() || c == '.')
          .unwrap_or(false)
        {
          let mut lookahead = chars.clone();
          lookahead.next(); // skip the dot
          let mut digits = 0usize;
          while let Some(next) = lookahead.peek() {
            if next.is_ascii_digit() {
              digits += 1;
              lookahead.next();
            } else {
              break;
            }
          }
          if digits > 0 && digits <= 2 {
            chars.next(); // consume dot
            output.push('.');
            prev = Some('.');
            continue;
          }
        }
      }
    }

    output.push(ch);
    prev = Some(ch);
  }

  output
}

fn strip_trailing_decimal_zeros(value: &str) -> String {
  let bytes = value.as_bytes();
  if bytes.is_empty() {
    return String::new();
  }

  let mut output = String::with_capacity(value.len());
  let mut index = 0usize;
  let mut inside_single_quote = false;
  let mut inside_double_quote = false;

  while index < bytes.len() {
    let current = bytes[index];

    if current == b'\'' && !inside_double_quote {
      inside_single_quote = !inside_single_quote;
      output.push('\'');
      index += 1;
      continue;
    }
    if current == b'"' && !inside_single_quote {
      inside_double_quote = !inside_double_quote;
      output.push('"');
      index += 1;
      continue;
    }
    if inside_single_quote || inside_double_quote {
      output.push(current as char);
      index += 1;
      continue;
    }

    if is_number_start(bytes, index) {
      let start = index;
      if matches!(bytes[index], b'+' | b'-') {
        index += 1;
      }
      while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
      }
      if index < bytes.len() && bytes[index] == b'.' {
        let integer_end = index;
        index += 1;
        let fractional_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
          index += 1;
        }
        if fractional_start == index {
          output.push_str(&value[start..index]);
          continue;
        }

        if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
          index += 1;
          if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
          }
          let mut has_exponent_digit = false;
          while index < bytes.len() && bytes[index].is_ascii_digit() {
            has_exponent_digit = true;
            index += 1;
          }
          if has_exponent_digit {
            output.push_str(&value[start..index]);
            continue;
          }
          output.push_str(&value[start..index]);
          continue;
        }

        let fraction = &value[fractional_start..index];
        let trimmed_fraction = fraction.trim_end_matches('0');
        if trimmed_fraction.is_empty() {
          let integer_part = &value[start..integer_end];
          let normalized_integer = if integer_part.is_empty()
            || integer_part == "+"
            || integer_part == "-"
            || integer_part == "+0"
            || integer_part == "-0"
            || integer_part.trim_matches('0').is_empty()
          {
            "0"
          } else {
            integer_part
          };
          output.push_str(normalized_integer);
          continue;
        }

        if trimmed_fraction.len() < fraction.len() {
          output.push_str(&value[start..integer_end]);
          output.push('.');
          output.push_str(trimmed_fraction);
          continue;
        }
      }
      output.push_str(&value[start..index]);
      continue;
    }

    output.push(current as char);
    index += 1;
  }

  output
}

fn is_identifier_continue(byte: u8) -> bool {
  byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

fn is_number_boundary(bytes: &[u8], index: usize) -> bool {
  if index == 0 {
    return true;
  }
  let prev = bytes[index - 1];
  if (prev as char).is_ascii_whitespace() {
    return true;
  }
  match prev {
    b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b'/' | b'*' | b'+' | b':' | b';' | b'='
    | b'<' | b'>' | b'!' | b'^' | b'&' | b'|' | b'~' | b'?' | b'%' => true,
    b'-' => {
      if index >= 2 {
        let before = bytes[index - 2];
        !is_identifier_continue(before)
      } else {
        true
      }
    }
    _ => false,
  }
}

fn is_number_start(bytes: &[u8], index: usize) -> bool {
  let current = bytes[index];
  let next = bytes.get(index + 1).copied();
  match current {
    b'+' | b'-' => {
      if !is_number_boundary(bytes, index) {
        return false;
      }
      matches!(next, Some(b'0'..=b'9') | Some(b'.'))
    }
    b'.' => {
      if !is_number_boundary(bytes, index) {
        return false;
      }
      matches!(next, Some(b'0'..=b'9'))
    }
    b'0'..=b'9' => {
      if index > 0 {
        let prev = bytes[index - 1];
        if prev.is_ascii_digit() || prev == b'.' {
          return false;
        }
      }
      is_number_boundary(bytes, index)
    }
    _ => false,
  }
}

fn convert_length_units(value: &str) -> String {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return String::new();
  }

  let mut core = trimmed;
  let mut important_suffix = "";

  if let Some(stripped) = core.strip_suffix("!important") {
    core = stripped.trim_end();
    important_suffix = "!important";
  }

  let bytes = core.as_bytes();
  let mut search_start = 0usize;
  let mut last_written = 0usize;
  let mut output = String::with_capacity(core.len());

  while let Some(rel_pos) = core[search_start..].find("px") {
    let px_index = search_start + rel_pos;
    let mut cursor = px_index;
    let mut has_digit = false;
    let mut seen_decimal = false;

    while cursor > 0 {
      let ch = bytes[cursor - 1] as char;
      if ch.is_ascii_digit() {
        has_digit = true;
        cursor -= 1;
        continue;
      }
      if ch == '.' && !seen_decimal {
        seen_decimal = true;
        cursor -= 1;
        continue;
      }
      if (ch == '+' || ch == '-') && cursor - 1 < px_index {
        if cursor > 1 {
          let prev = bytes[cursor - 2] as char;
          if prev.is_ascii_alphanumeric() || prev == '_' {
            break;
          }
        }
        cursor -= 1;
        continue;
      }
      break;
    }

    if !has_digit {
      search_start = px_index + 2;
      continue;
    }

    let mut number_start = cursor;
    while number_start < px_index {
      let ch = core.as_bytes()[number_start] as char;
      if ch.is_ascii_whitespace() {
        number_start += 1;
      } else {
        break;
      }
    }

    if number_start >= px_index {
      search_start = px_index + 2;
      continue;
    }

    if number_start > 0 {
      let prev = bytes[number_start - 1] as char;
      let mut should_skip = prev.is_ascii_alphanumeric() || prev == '_';
      if !should_skip && prev == '-' && number_start >= 2 {
        let prev2 = bytes[number_start - 2] as char;
        if prev2.is_ascii_alphanumeric() || prev2 == '_' || prev2 == '-' {
          should_skip = true;
        }
      }
      if should_skip {
        search_start = px_index + 2;
        continue;
      }
    }

    let number_str = &core[number_start..px_index];
    if number_str.contains('.') || number_str.contains('e') || number_str.contains('E') {
      search_start = px_index + 2;
      continue;
    }

    if let Ok(px_value) = number_str.parse::<i64>() {
      let mut best: Option<String> = None;

      if px_value != 0 {
        if px_value % 96 == 0 {
          let converted_abs = px_value.abs() / 96;
          let mut candidate = String::new();
          if px_value < 0 {
            candidate.push('-');
          }
          candidate.push_str(&converted_abs.to_string());
          candidate.push_str("in");
          best = match best {
            Some(existing) => {
              if candidate.len() < existing.len() {
                Some(candidate)
              } else {
                Some(existing)
              }
            }
            None => Some(candidate),
          };
        }

        if px_value % 16 == 0 {
          let converted_abs = px_value.abs() / 16;
          let mut candidate = String::new();
          if px_value < 0 {
            candidate.push('-');
          }
          candidate.push_str(&converted_abs.to_string());
          candidate.push_str("pc");
          best = match best {
            Some(existing) => {
              if candidate.len() < existing.len() {
                Some(candidate)
              } else {
                Some(existing)
              }
            }
            None => Some(candidate),
          };
        }

        if px_value % 4 == 0 {
          let pt_value = px_value * 3 / 4;
          let mut candidate = String::new();
          if pt_value < 0 {
            candidate.push('-');
          }
          candidate.push_str(&pt_value.abs().to_string());
          candidate.push_str("pt");
          best = match best {
            Some(existing) => {
              if candidate.len() < existing.len() {
                Some(candidate)
              } else {
                Some(existing)
              }
            }
            None => Some(candidate),
          };
        }
      }

      if let Some(candidate) = best {
        let original_len = px_index + 2 - number_start;
        if candidate.len() < original_len {
          output.push_str(&core[last_written..number_start]);
          output.push_str(&candidate);
          last_written = px_index + 2;
        }
      }
    }

    search_start = px_index + 2;
  }

  if last_written == 0 {
    if important_suffix.is_empty() {
      core.to_string()
    } else {
      let mut result = core.to_string();
      result.push_str(important_suffix);
      result
    }
  } else {
    output.push_str(&core[last_written..]);
    if important_suffix.is_empty() {
      output
    } else {
      output.push_str(important_suffix);
      output
    }
  }
}

fn convert_time_units(value: &str) -> String {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return String::new();
  }

  let mut core = trimmed;
  let mut important_suffix = "";
  if let Some(stripped) = core.strip_suffix("!important") {
    core = stripped.trim_end();
    important_suffix = "!important";
  }

  let bytes = core.as_bytes();
  let mut search_start = 0usize;
  let mut last_written = 0usize;
  let mut output = String::with_capacity(core.len());

  while let Some(rel_pos) = core[search_start..].find("ms") {
    let ms_index = search_start + rel_pos;
    let mut cursor = ms_index;
    let mut has_digit = false;
    let mut seen_decimal = false;

    while cursor > 0 {
      let ch = bytes[cursor - 1] as char;
      if ch.is_ascii_digit() {
        has_digit = true;
        cursor -= 1;
        continue;
      }
      if ch == '.' {
        seen_decimal = true;
        cursor -= 1;
        continue;
      }
      if (ch == '+' || ch == '-') && cursor - 1 < ms_index {
        cursor -= 1;
        break;
      }
      break;
    }

    if !has_digit || seen_decimal {
      search_start = ms_index + 2;
      continue;
    }

    if cursor > 0 {
      let prev = bytes[cursor - 1] as char;
      if prev.is_ascii_alphabetic() || prev == '_' || prev == '-' {
        search_start = ms_index + 2;
        continue;
      }
    }

    let number_str = &core[cursor..ms_index];
    if number_str.contains(',') || number_str.contains(' ') {
      search_start = ms_index + 2;
      continue;
    }

    if let Ok(ms_value) = number_str.parse::<i64>() {
      let seconds = ms_value as f64 / 1000.0;
      let mut seconds_str = format!("{}", seconds);
      if seconds_str.contains('.') {
        while seconds_str.ends_with('0') {
          seconds_str.pop();
        }
        if seconds_str.ends_with('.') {
          seconds_str.push('0');
        }
      }
      if seconds_str.starts_with("0.") {
        seconds_str.remove(0);
      } else if seconds_str.starts_with("-0.") {
        seconds_str.remove(1);
      }
      let candidate = format!("{}s", seconds_str);
      let original_len = ms_index + 2 - cursor;
      if candidate.len() < original_len {
        output.push_str(&core[last_written..cursor]);
        output.push_str(&candidate);
        last_written = ms_index + 2;
      }
    }

    search_start = ms_index + 2;
  }

  if last_written == 0 {
    if important_suffix.is_empty() {
      core.to_string()
    } else {
      let mut result = core.to_string();
      result.push_str(important_suffix);
      result
    }
  } else {
    output.push_str(&core[last_written..]);
    if important_suffix.is_empty() {
      output
    } else {
      output.push_str(important_suffix);
      output
    }
  }
}

fn strip_zero_units(value: &str) -> String {
  const UNITS: [&str; 7] = ["px", "em", "rem", "vw", "vh", "vmin", "vmax"];

  let bytes = value.as_bytes();
  let mut index = 0usize;
  let mut output = String::with_capacity(value.len());

  while index < bytes.len() {
    let current = bytes[index];
    if current == b'0' {
      let prev_byte = if index > 0 {
        Some(bytes[index - 1])
      } else {
        None
      };
      let prev_is_digit_or_dot = prev_byte
        .map(|byte| {
          let ch = byte as char;
          ch.is_ascii_digit() || ch == '.'
        })
        .unwrap_or(false);

      if !prev_is_digit_or_dot {
        let mut replaced = false;
        for unit in UNITS {
          let unit_bytes = unit.as_bytes();
          if index + 1 + unit_bytes.len() <= bytes.len()
            && &bytes[index + 1..index + 1 + unit_bytes.len()] == unit_bytes
          {
            if prev_byte == Some(b'-') && output.ends_with('-') {
              output.pop();
            }
            output.push('0');
            index += 1 + unit_bytes.len();
            replaced = true;
            break;
          }
        }

        if replaced {
          continue;
        }
      }
    }

    output.push(current as char);
    index += 1;
  }

  output
}

fn zero_with_alpha_unit(raw: &str) -> Option<String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() || trimmed.starts_with(['-', '+']) {
    return None;
  }
  let mut chars = trimmed.chars();
  let first = chars.next()?;
  if first != '0' {
    return None;
  }
  let suffix = chars.as_str();
  if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_alphabetic()) {
    return None;
  }
  Some(format!("0{}", suffix.to_ascii_lowercase()))
}

fn parse_rgb_component(component: &str) -> Option<u8> {
  let trimmed = component.trim();
  if trimmed.is_empty() {
    return None;
  }
  if let Some(stripped) = trimmed.strip_suffix('%') {
    let value: f64 = stripped.trim().parse().ok()?;
    let clamped = value.max(0.0).min(100.0);
    Some((clamped * 255.0 / 100.0).round().max(0.0).min(255.0) as u8)
  } else {
    let value: f64 = trimmed.parse().ok()?;
    let clamped = value.max(0.0).min(255.0);
    Some(clamped.round().max(0.0).min(255.0) as u8)
  }
}

fn parse_alpha_component(component: &str) -> Option<u8> {
  let trimmed = component.trim();
  if trimmed.is_empty() {
    return None;
  }
  if let Some(stripped) = trimmed.strip_suffix('%') {
    let value: f64 = stripped.trim().parse().ok()?;
    let clamped = value.max(0.0).min(100.0);
    Some((clamped * 255.0 / 100.0).round().max(0.0).min(255.0) as u8)
  } else {
    let value: f64 = trimmed.parse().ok()?;
    let clamped = value.max(0.0).min(1.0);
    Some((clamped * 255.0).round().max(0.0).min(255.0) as u8)
  }
}

fn convert_rgb_like_to_hex(segment: &str, is_rgba: bool) -> Option<(String, usize)> {
  let prefix_len = if is_rgba { 4 } else { 3 };
  if segment.len() <= prefix_len || !segment.as_bytes()[prefix_len].eq(&b'(') {
    return None;
  }
  let start = prefix_len + 1;
  let end_rel = segment[start..].find(')')?;
  let end = start + end_rel;
  let inner = &segment[start..end];
  let consumed = end + 1;
  let components: Vec<&str> = inner.split(',').map(|part| part.trim()).collect();
  if is_rgba && components.len() != 4 {
    return None;
  }
  if !is_rgba && components.len() != 3 {
    return None;
  }

  let r = parse_rgb_component(components.get(0)?.trim())?;
  let g = parse_rgb_component(components.get(1)?.trim())?;
  let b = parse_rgb_component(components.get(2)?.trim())?;

  if is_rgba {
    let a = parse_alpha_component(components.get(3)?.trim())?;
    if a == 255 {
      Some((format!("#{:02x}{:02x}{:02x}", r, g, b), consumed))
    } else {
      Some((format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a), consumed))
    }
  } else {
    Some((format!("#{:02x}{:02x}{:02x}", r, g, b), consumed))
  }
}

fn convert_color_functions_to_hex(value: &str) -> String {
  let mut output = String::with_capacity(value.len());
  let mut index = 0usize;

  while index < value.len() {
    let rest = &value[index..];
    let mut consumed = 0usize;

    let rest_bytes = rest.as_bytes();
    if rest_bytes.len() >= 5
      && rest_bytes[4] == b'('
      && starts_with_ignore_ascii(rest_bytes, b"rgba")
    {
      if let Some(result) = convert_rgb_like_to_hex(rest, true) {
        output.push_str(&result.0);
        consumed = result.1;
      }
    } else if rest_bytes.len() >= 4
      && rest_bytes[3] == b'('
      && starts_with_ignore_ascii(rest_bytes, b"rgb")
    {
      if let Some(result) = convert_rgb_like_to_hex(rest, false) {
        output.push_str(&result.0);
        consumed = result.1;
      }
    }

    if consumed > 0 {
      index += consumed;
      continue;
    }

    let mut chars = rest.chars();
    if let Some(ch) = chars.next() {
      output.push(ch);
      index += ch.len_utf8();
    } else {
      break;
    }
  }

  output
}

fn canonicalize_selector_key(selector: &str) -> String {
  let mut output = String::with_capacity(selector.len());
  let mut chars = selector.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '.' && matches!(chars.peek(), Some('_')) {
      output.push_str("._HASH");
      chars.next();
      while let Some(next) = chars.peek() {
        if next.is_ascii_alphanumeric() || *next == '_' {
          chars.next();
        } else {
          break;
        }
      }
      continue;
    }
    output.push(ch);
  }

  output
}

fn vendor_prefixed_values(property: &str, value: &str) -> Option<Vec<String>> {
  let normalized_value = value.trim();
  if normalized_value.is_empty() {
    return None;
  }

  let property_lower = property.to_ascii_lowercase();
  let applies_to_fit_content = matches!(
    property_lower.as_str(),
    "width" | "height" | "min-width" | "max-width" | "min-height" | "max-height"
  );

  if applies_to_fit_content && normalized_value == "fit-content" {
    if normalized_value.contains("-moz-fit-content") {
      return None;
    }
    return Some(vec!["-moz-fit-content".into(), "fit-content".into()]);
  }

  if applies_to_fit_content && normalized_value == "fill" {
    return Some(vec!["-webkit-fill-available".into(), "fill".into()]);
  }

  None
}

struct PropertyExpansion {
  name: String,
  raw_value: String,
}

fn should_skip_shorthand_expansion(raw_value: &str) -> bool {
  raw_value.to_ascii_lowercase().contains("var(")
}

fn split_css_value_components(raw_value: &str) -> Vec<String> {
  let mut parts = Vec::new();
  let mut current = String::new();
  let mut paren_depth = 0usize;
  let mut bracket_depth = 0usize;
  let mut brace_depth = 0usize;
  let mut string_delim: Option<char> = None;
  let mut escape_next = false;

  for ch in raw_value.chars() {
    if let Some(delim) = string_delim {
      current.push(ch);
      if escape_next {
        escape_next = false;
        continue;
      }
      if ch == '\\' {
        escape_next = true;
      } else if ch == delim {
        string_delim = None;
      }
      continue;
    }

    match ch {
      '"' | '\'' => {
        string_delim = Some(ch);
        current.push(ch);
      }
      '(' => {
        paren_depth += 1;
        current.push(ch);
      }
      ')' => {
        if paren_depth > 0 {
          paren_depth -= 1;
        }
        current.push(ch);
      }
      '[' => {
        bracket_depth += 1;
        current.push(ch);
      }
      ']' => {
        if bracket_depth > 0 {
          bracket_depth -= 1;
        }
        current.push(ch);
      }
      '{' => {
        brace_depth += 1;
        current.push(ch);
      }
      '}' => {
        if brace_depth > 0 {
          brace_depth -= 1;
        }
        current.push(ch);
      }
      ' ' | '\t' | '\n' | '\r' => {
        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
          if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
            current.clear();
          }
        } else {
          current.push(ch);
        }
      }
      _ => current.push(ch),
    }
  }

  if !current.trim().is_empty() {
    parts.push(current.trim().to_string());
  }

  parts
}

fn expand_box_shorthand(property: &str, raw_value: &str) -> Vec<PropertyExpansion> {
  if should_skip_shorthand_expansion(raw_value) {
    return vec![PropertyExpansion {
      name: property.to_string(),
      raw_value: raw_value.to_string(),
    }];
  }

  let values = split_css_value_components(raw_value);
  if values.is_empty() || values.len() > 4 {
    return vec![PropertyExpansion {
      name: property.to_string(),
      raw_value: raw_value.to_string(),
    }];
  }

  let top = values[0].clone();
  let right = values.get(1).cloned().unwrap_or_else(|| top.clone());
  let bottom = values.get(2).cloned().unwrap_or_else(|| top.clone());
  let left = values.get(3).cloned().unwrap_or_else(|| right.clone());

  vec![
    PropertyExpansion {
      name: format!("{}-top", property),
      raw_value: top,
    },
    PropertyExpansion {
      name: format!("{}-right", property),
      raw_value: right,
    },
    PropertyExpansion {
      name: format!("{}-bottom", property),
      raw_value: bottom,
    },
    PropertyExpansion {
      name: format!("{}-left", property),
      raw_value: left,
    },
  ]
}

const OUTLINE_STYLE_VALUES: &[&str] = &[
  "auto", "none", "dotted", "dashed", "solid", "double", "groove", "ridge", "inset", "outset",
];

const OUTLINE_GLOBAL_VALUES: &[&str] = &["inherit", "initial", "unset", "revert", "revert-layer"];

const OUTLINE_WIDTH_UNITS: &[&str] = &[
  "%", "cap", "ch", "cm", "em", "ex", "fr", "ic", "in", "lh", "mm", "pc", "pt", "px", "q", "rem",
  "rlh", "vb", "vh", "vi", "vmax", "vmin", "vw",
];

fn is_outline_style_value(value: &str) -> bool {
  let lower = value.trim().to_ascii_lowercase();
  OUTLINE_STYLE_VALUES
    .iter()
    .any(|candidate| lower == *candidate)
    || OUTLINE_GLOBAL_VALUES
      .iter()
      .any(|candidate| lower == *candidate)
}

fn is_outline_width_value(value: &str) -> bool {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return false;
  }
  let lower = trimmed.to_ascii_lowercase();
  if OUTLINE_GLOBAL_VALUES
    .iter()
    .any(|candidate| lower == *candidate)
  {
    return true;
  }
  if matches!(
    lower.as_str(),
    "auto" | "thin" | "medium" | "thick" | "min-content" | "max-content" | "fit-content"
  ) {
    return true;
  }
  if trimmed.contains('(') {
    return true;
  }
  let mut chars = trimmed.chars().peekable();
  if matches!(chars.peek(), Some(c) if *c == '+' || *c == '-') {
    chars.next();
  }
  let mut has_digit = false;
  while let Some(ch) = chars.peek() {
    if ch.is_ascii_digit() {
      has_digit = true;
      chars.next();
      continue;
    }
    if *ch == '.' {
      chars.next();
      continue;
    }
    break;
  }
  if !has_digit {
    return false;
  }
  let unit: String = chars.collect();
  if unit.is_empty() {
    return true;
  }
  let lower_unit = unit.trim().to_ascii_lowercase();
  OUTLINE_WIDTH_UNITS
    .iter()
    .any(|candidate| lower_unit == *candidate)
}

fn is_outline_color_value(value: &str) -> bool {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return false;
  }
  let lower = trimmed.to_ascii_lowercase();
  if OUTLINE_GLOBAL_VALUES
    .iter()
    .any(|candidate| lower == *candidate)
  {
    return true;
  }
  if lower == "transparent" || lower == "currentcolor" {
    return true;
  }
  if lower.starts_with('#') {
    return lower.chars().skip(1).all(|ch| ch.is_ascii_hexdigit());
  }
  if named_color_hex(&lower).is_some() {
    return true;
  }
  if let Some(index) = lower.find('(') {
    let func = &lower[..index];
    return matches!(
      func,
      "rgb"
        | "rgba"
        | "hsl"
        | "hsla"
        | "hwb"
        | "lab"
        | "lch"
        | "color"
        | "device-cmyk"
        | "oklab"
        | "oklch"
    );
  }
  false
}

fn expand_outline_shorthand(raw_value: &str) -> Option<Vec<PropertyExpansion>> {
  if should_skip_shorthand_expansion(raw_value) {
    return None;
  }

  let values = split_css_value_components(raw_value);
  if values.len() > 3 {
    return None;
  }

  let mut color_value: Option<String> = None;
  let mut style_value: Option<String> = None;
  let mut width_value: Option<String> = None;

  for value in values {
    if is_outline_color_value(&value) {
      if color_value.is_some() {
        return None;
      }
      color_value = Some(value);
      continue;
    }
    if is_outline_style_value(&value) {
      if style_value.is_some() {
        return None;
      }
      style_value = Some(value);
      continue;
    }
    if is_outline_width_value(&value) {
      if width_value.is_some() {
        return None;
      }
      width_value = Some(value);
      continue;
    }
    return None;
  }

  let color = color_value.unwrap_or_else(|| "currentColor".to_string());
  let style = style_value.unwrap_or_else(|| "none".to_string());
  let width = width_value.unwrap_or_else(|| "medium".to_string());

  Some(vec![
    PropertyExpansion {
      name: "outline-color".into(),
      raw_value: color,
    },
    PropertyExpansion {
      name: "outline-style".into(),
      raw_value: style,
    },
    PropertyExpansion {
      name: "outline-width".into(),
      raw_value: width,
    },
  ])
}

fn is_text_decoration_color_token(value: &str) -> bool {
  let lower = value.to_ascii_lowercase();
  if lower.starts_with('#') && lower.len() > 1 && lower[1..].chars().all(|c| c.is_ascii_hexdigit())
  {
    return true;
  }
  if named_color_hex(lower.as_str()).is_some() {
    return true;
  }
  if matches!(lower.as_str(), "transparent" | "currentcolor") {
    return true;
  }
  if lower.starts_with("rgb(")
    || lower.starts_with("rgba(")
    || lower.starts_with("hsl(")
    || lower.starts_with("hsla(")
    || lower.starts_with("hwb(")
    || lower.starts_with("lab(")
    || lower.starts_with("lch(")
    || lower.starts_with("oklab(")
    || lower.starts_with("oklch(")
    || lower.starts_with("color(")
    || lower.starts_with("var(")
  {
    return true;
  }
  false
}

fn expand_text_decoration(raw_value: &str) -> Option<Vec<PropertyExpansion>> {
  let trimmed = raw_value.trim();
  if trimmed.is_empty() {
    return Some(Vec::new());
  }

  const GLOBAL_VALUES: &[&str] = &["inherit", "initial", "unset", "revert", "revert-layer"];
  const LINE_KEYWORDS: &[&str] = &["none", "underline", "overline", "line-through", "blink"];
  const STYLE_KEYWORDS: &[&str] = &["solid", "double", "dotted", "dashed", "wavy"];

  let mut line_values: Vec<String> = Vec::new();
  let mut style_value: Option<String> = None;
  let mut color_value: Option<String> = None;

  for token in trimmed.split_whitespace() {
    let lower = token.to_ascii_lowercase();
    if LINE_KEYWORDS.contains(&lower.as_str()) || GLOBAL_VALUES.contains(&lower.as_str()) {
      if line_values.contains(&lower) {
        return Some(Vec::new());
      }
      line_values.push(lower);
      continue;
    }
    if STYLE_KEYWORDS.contains(&lower.as_str()) || GLOBAL_VALUES.contains(&lower.as_str()) {
      if style_value.is_some() {
        return Some(Vec::new());
      }
      style_value = Some(lower);
      continue;
    }
    if is_text_decoration_color_token(&lower) {
      if color_value.is_some() {
        return Some(Vec::new());
      }
      let normalized = if lower == "currentcolor" {
        "currentColor".to_string()
      } else {
        lower
      };
      color_value = Some(normalized);
      continue;
    }
    return Some(Vec::new());
  }

  line_values.sort();
  let resolved_line = if line_values.is_empty() {
    "none".to_string()
  } else {
    line_values.join(" ")
  };
  let color = color_value.unwrap_or_else(|| "initial".into());
  let style = style_value.unwrap_or_else(|| "solid".into());

  Some(vec![
    PropertyExpansion {
      name: "text-decoration-color".into(),
      raw_value: color,
    },
    PropertyExpansion {
      name: "text-decoration-line".into(),
      raw_value: resolved_line,
    },
    PropertyExpansion {
      name: "text-decoration-style".into(),
      raw_value: style,
    },
  ])
}

fn expand_property(property: &str, raw_value: &str) -> Vec<PropertyExpansion> {
  if property == "flex" {
    let trimmed = raw_value.trim();
    if trimmed == "1" {
      return vec![
        PropertyExpansion {
          name: "flex-grow".into(),
          raw_value: "1".into(),
        },
        PropertyExpansion {
          name: "flex-shrink".into(),
          raw_value: "1".into(),
        },
        PropertyExpansion {
          name: "flex-basis".into(),
          raw_value: "0%".into(),
        },
      ];
    }
    if trimmed.eq_ignore_ascii_case("none") {
      return vec![
        PropertyExpansion {
          name: "flex-grow".into(),
          raw_value: "0".into(),
        },
        PropertyExpansion {
          name: "flex-shrink".into(),
          raw_value: "0".into(),
        },
        PropertyExpansion {
          name: "flex-basis".into(),
          raw_value: "auto".into(),
        },
      ];
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() == 3 {
      return vec![
        PropertyExpansion {
          name: "flex-grow".into(),
          raw_value: parts[0].to_string(),
        },
        PropertyExpansion {
          name: "flex-shrink".into(),
          raw_value: parts[1].to_string(),
        },
        PropertyExpansion {
          name: "flex-basis".into(),
          raw_value: parts[2].to_string(),
        },
      ];
    }
  }

  if property == "user-select" {
    return vec![
      PropertyExpansion {
        name: "-webkit-user-select".into(),
        raw_value: raw_value.to_string(),
      },
      PropertyExpansion {
        name: property.to_string(),
        raw_value: raw_value.to_string(),
      },
    ];
  }

  if property == "flex-flow" {
    let mut direction: Option<String> = None;
    let mut wrap: Option<String> = None;
    for part in raw_value.split_whitespace() {
      let lower = part.to_ascii_lowercase();
      match lower.as_str() {
        "row" | "row-reverse" | "column" | "column-reverse" => {
          if direction.is_some() {
            return vec![PropertyExpansion {
              name: property.to_string(),
              raw_value: raw_value.to_string(),
            }];
          }
          direction = Some(lower);
        }
        "nowrap" | "wrap" | "wrap-reverse" => {
          if wrap.is_some() {
            return vec![PropertyExpansion {
              name: property.to_string(),
              raw_value: raw_value.to_string(),
            }];
          }
          wrap = Some(lower);
        }
        _ => {
          return vec![PropertyExpansion {
            name: property.to_string(),
            raw_value: raw_value.to_string(),
          }];
        }
      }
    }

    let mut expansions = Vec::new();
    if let Some(direction) = direction {
      expansions.push(PropertyExpansion {
        name: "flex-direction".into(),
        raw_value: direction,
      });
    }
    if let Some(wrap) = wrap {
      expansions.push(PropertyExpansion {
        name: "flex-wrap".into(),
        raw_value: wrap,
      });
    }
    if expansions.is_empty() {
      return vec![PropertyExpansion {
        name: property.to_string(),
        raw_value: raw_value.to_string(),
      }];
    }
    return expansions;
  }

  if property == "text-decoration" {
    if let Some(expanded) = expand_text_decoration(raw_value) {
      return expanded;
    }
  }

  if property == "padding" || property == "margin" {
    return expand_box_shorthand(property, raw_value);
  }

  if property == "outline" {
    if let Some(expanded) = expand_outline_shorthand(raw_value) {
      return expanded;
    }
  }

  if let Some(names) = expand_shorthand_properties(property) {
    return names
      .into_iter()
      .map(|name| PropertyExpansion {
        raw_value: raw_value.to_string(),
        name,
      })
      .collect();
  }

  vec![PropertyExpansion {
    name: property.to_string(),
    raw_value: raw_value.to_string(),
  }]
}

fn expand_shorthand_properties(property: &str) -> Option<Vec<String>> {
  match property {
    "overflow" => Some(vec!["overflow-x".into(), "overflow-y".into()]),
    _ => None,
  }
}

pub(crate) fn shorthand_bucket(property: &str) -> Option<u16> {
  match property {
    "all" => Some(0),
    "animation"
    | "animation-range"
    | "background"
    | "border"
    | "border-image"
    | "border-radius"
    | "column-rule"
    | "columns"
    | "contain-intrinsic-size"
    | "container"
    | "flex"
    | "flex-flow"
    | "font"
    | "font-synthesis"
    | "gap"
    | "grid"
    | "grid-area"
    | "grid-template"
    | "inset"
    | "list-style"
    | "margin"
    | "mask"
    | "mask-border"
    | "offset"
    | "outline"
    | "overflow"
    | "overscroll-behavior"
    | "padding"
    | "place-content"
    | "place-items"
    | "place-self"
    | "position-try"
    | "scroll-margin"
    | "scroll-padding"
    | "scroll-timeline"
    | "text-decoration"
    | "text-emphasis"
    | "text-wrap"
    | "transition"
    | "view-timeline" => Some(1),
    "border-color"
    | "border-style"
    | "border-width"
    | "grid-column"
    | "grid-row"
    | "inset-block"
    | "inset-inline"
    | "margin-block"
    | "margin-inline"
    | "padding-block"
    | "padding-inline"
    | "scroll-margin-block"
    | "scroll-margin-inline"
    | "scroll-padding-block"
    | "scroll-padding-inline"
    | "font-variant" => Some(2),
    "border-block" | "border-inline" => Some(3),
    "border-top" | "border-right" | "border-bottom" | "border-left" => Some(4),
    "border-block-start" | "border-block-end" | "border-inline-start" | "border-inline-end" => {
      Some(5)
    }
    _ => None,
  }
}

fn trim_numeric(value: &str) -> String {
  value.trim().to_string()
}

fn is_unitless_property(name: &str) -> bool {
  matches!(
    name,
    "animationIterationCount"
      | "basePalette"
      | "borderImageOutset"
      | "borderImageSlice"
      | "borderImageWidth"
      | "boxFlex"
      | "boxFlexGroup"
      | "boxOrdinalGroup"
      | "columnCount"
      | "columns"
      | "flex"
      | "flexGrow"
      | "flexPositive"
      | "flexShrink"
      | "flexNegative"
      | "flexOrder"
      | "fontSizeAdjust"
      | "fontWeight"
      | "gridArea"
      | "gridRow"
      | "gridRowEnd"
      | "gridRowSpan"
      | "gridRowStart"
      | "gridColumn"
      | "gridColumnEnd"
      | "gridColumnSpan"
      | "gridColumnStart"
      | "lineClamp"
      | "lineHeight"
      | "opacity"
      | "order"
      | "orphans"
      | "tabSize"
      | "WebkitLineClamp"
      | "widows"
      | "zIndex"
      | "zoom"
      | "fillOpacity"
      | "floodOpacity"
      | "stopOpacity"
      | "strokeDasharray"
      | "strokeDashoffset"
      | "strokeMiterlimit"
      | "strokeOpacity"
      | "strokeWidth"
  )
}

fn replace_nesting(selector: &str, class_name: &str) -> String {
  selector
    .replace('&', &format!(".{}", class_name))
    .replace(" >[", ">[")
    .replace(" +[", "+[")
    .replace(" ~[", "~[")
}

fn minify_selector(selector: &str) -> String {
  let mut result = String::with_capacity(selector.len());
  let mut chars = selector.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch.is_ascii_whitespace() {
      while matches!(chars.peek(), Some(next) if next.is_ascii_whitespace()) {
        chars.next();
      }
      let mut prev_non_whitespace = None;
      for ch in result.chars().rev() {
        if !ch.is_ascii_whitespace() {
          prev_non_whitespace = Some(ch);
          break;
        }
      }
      let prev_is_combinator = prev_non_whitespace
        .map(|c| matches!(c, '>' | '+' | '~' | ','))
        .unwrap_or(false);
      let next_is_combinator = chars
        .peek()
        .map(|c| matches!(c, '>' | '+' | '~' | ','))
        .unwrap_or(false);
      if prev_non_whitespace == Some('&') && next_is_combinator {
        let mut lookahead = chars.clone();
        lookahead.next();
        while matches!(lookahead.peek(), Some(next) if next.is_ascii_whitespace()) {
          lookahead.next();
        }
        let next_non_whitespace = lookahead.peek().copied();
        if matches!(next_non_whitespace, Some(':') | Some('*')) {
          result.push(' ');
        }
        continue;
      }
      if prev_is_combinator || next_is_combinator {
        continue;
      }
      result.push(' ');
    } else if matches!(ch, '>' | '+' | '~' | ',') {
      while result.ends_with(' ') {
        let trimmed = result.trim_end_matches(' ');
        if trimmed.chars().last() == Some('&') {
          break;
        }
        result.pop();
      }
      result.push(ch);
      while matches!(chars.peek(), Some(next) if next.is_ascii_whitespace()) {
        chars.next();
      }
    } else {
      result.push(ch);
    }
  }

  result
}

fn join_selectors(selectors: &[String], class_name: &str) -> String {
  selectors
    .iter()
    .map(|selector| replace_nesting(selector, class_name))
    .collect::<Vec<_>>()
    .join(",")
}

fn apply_increase_specificity(selector: &str) -> String {
  if !selector.contains("._") {
    return selector.to_string();
  }

  let mut result = String::with_capacity(selector.len() + 16);
  let chars: Vec<(usize, char)> = selector.char_indices().collect();
  let mut index = 0usize;

  while index < chars.len() {
    let (byte_index, ch) = chars[index];
    result.push(ch);

    if ch == '.' {
      let mut class_chars = String::new();
      let mut next = index + 1;
      while next < chars.len() {
        let (_, next_ch) = chars[next];
        if next_ch.is_ascii_alphanumeric() || next_ch == '_' || next_ch == '-' {
          result.push(next_ch);
          class_chars.push(next_ch);
          next += 1;
        } else {
          break;
        }
      }

      if !class_chars.is_empty() && class_chars.starts_with('_') {
        let class_end_byte = if next < chars.len() {
          chars[next].0
        } else {
          selector.len()
        };
        if !selector[class_end_byte..].starts_with(INCREASE_SPECIFICITY_SELECTOR) {
          result.push_str(INCREASE_SPECIFICITY_SELECTOR);
        }
      }

      index = next;
      continue;
    }

    if ch.is_ascii() {
      // advance using prepared index when we didn't consume additional chars.
      index += 1;
    } else {
      // fallback for multi-byte characters: find next index by matching byte offset.
      let mut next_index = index + 1;
      while next_index < chars.len() && chars[next_index].0 == byte_index {
        next_index += 1;
      }
      index = next_index;
    }
  }

  result
}

pub(crate) fn minify_at_rule_params(raw: &str) -> String {
  let mut result = String::with_capacity(raw.len());
  let mut chars = raw.chars().peekable();
  let mut pending_space = false;

  while let Some(ch) = chars.next() {
    if ch.is_ascii_whitespace() {
      pending_space = true;
      continue;
    }
    if pending_space {
      if !result.ends_with('(') && !result.is_empty() {
        result.push(' ');
      }
      pending_space = false;
    }
    result.push(ch);
    if ch == ':' {
      while matches!(chars.peek(), Some(next) if next.is_ascii_whitespace()) {
        chars.next();
      }
    }
  }

  result.trim().to_string()
}

pub(crate) fn wrap_at_rules(mut css: String, at_rules: &[AtRuleInput]) -> String {
  for at_rule in at_rules.iter().rev() {
    let name = at_rule.name.trim();
    let params = minify_at_rule_params(&at_rule.params);
    if params.is_empty() {
      css = format!("@{}{{{}}}", name, css);
    } else {
      css = format!("@{} {}{{{}}}", name, params, css);
    }
  }
  css
}

pub fn atomicize_rules(rules: &[CssRuleInput], options: &CssOptions) -> CssArtifacts {
  let mut artifacts = CssArtifacts::default();
  let debug_hash = std::env::var_os("COMPILED_DEBUG_HASH").is_some();

  let mut indices: Vec<usize> = (0..rules.len()).collect();
  indices.sort_by(|&a, &b| {
    let rule_a = &rules[a];
    let rule_b = &rules[b];
    let bucket_a = shorthand_bucket(rule_a.property.as_str()).unwrap_or(u16::MAX);
    let bucket_b = shorthand_bucket(rule_b.property.as_str()).unwrap_or(u16::MAX);
    bucket_a.cmp(&bucket_b).then_with(|| a.cmp(&b))
  });

  for index in indices {
    let rule = &rules[index];
    if std::env::var_os("COMPILED_DEBUG_CSS").is_some()
      && rule.property == "flex-shrink"
      && rule
        .selectors
        .iter()
        .any(|selector| selector.contains(":is"))
    {
      eprintln!("[compiled-debug] raw selectors: {:?}", rule.selectors);
    }
    let normalized_selectors = if rule.selectors.is_empty() {
      vec![normalize_selector(None)]
    } else {
      rule
        .selectors
        .iter()
        .map(|selector| {
          let mut normalized = normalize_selector(Some(selector));
          if selector.contains(" >*") && normalized.contains(">*") {
            normalized = normalized.replace(">*", " >*");
          }
          if selector.contains(" >:") && normalized.contains(">:") {
            normalized = normalized.replace(">:", " >:");
          }
          if selector.contains(" >.") && normalized.contains(">.") {
            normalized = normalized.replace(">.", " >.");
          }
          if selector.contains(" >+") && normalized.contains(">+") {
            normalized = normalized.replace(">+", " >+");
          }
          if selector.contains(" >~") && normalized.contains(">~") {
            normalized = normalized.replace(">~", " >~");
          }
          normalized
        })
        .collect::<Vec<_>>()
    };

    let mut at_rule_label = if rule.at_rules.is_empty() {
      "undefined".to_string()
    } else {
      String::new()
    };
    if !rule.at_rules.is_empty() {
      for input in &rule.at_rules {
        at_rule_label.push_str(input.name.trim());
        at_rule_label.push_str(&minify_at_rule_params(&input.params));
      }
    }
    let prefix = options.class_hash_prefix.as_deref().unwrap_or("");
    let expansions = expand_property(rule.property.as_str(), &rule.raw_value);

    for expansion in expansions {
      if expansion.name == "-webkit-user-select" {
        continue;
      }
      let mut normalized = normalize_css_value(&expansion.raw_value);
      if expansion.name.starts_with("--") {
        if normalized.output_value == "0" && normalized.hash_value == "0" {
          if let Some(zero_unit_value) = zero_with_alpha_unit(&expansion.raw_value) {
            normalized.hash_value = zero_unit_value.clone();
            normalized.output_value = zero_unit_value;
          }
        }
      }
      if expansion.name == "flex-basis" {
        if let Some(px_value) = convert_pc_to_px(&normalized.output_value) {
          normalized.hash_value = px_value.clone();
          normalized.output_value = px_value;
        }
      } else if expansion.name == "transition" {
        let adjusted = normalize_transition_value(&normalized.output_value);
        normalized.hash_value = adjusted.hash_value;
        normalized.output_value = adjusted.output_value;
      } else if expansion.name == "background-position" {
        if let Some(adjusted_hash) = normalize_background_position(&normalized.hash_value) {
          normalized.hash_value = adjusted_hash;
        }
        if let Some(adjusted_output) = normalize_background_position(&normalized.output_value) {
          normalized.output_value = adjusted_output;
        }
      }

      let hash_value = normalized.hash_value.clone();
      let output_value = normalized.output_value.clone();

      let (declaration_values, value_for_hash) = if expansion.name == "user-select" {
        let value_with_flag = if rule.important {
          format!("{}!important", output_value)
        } else {
          output_value.clone()
        };
        let declarations = vec![
          format!("-webkit-user-select:{}", value_with_flag),
          format!("user-select:{}", value_with_flag),
        ];
        let hash_for_value = if rule.important {
          format!("{}!important", hash_value)
        } else {
          hash_value.clone()
        };
        (declarations, hash_for_value)
      } else {
        let vendor_values = vendor_prefixed_values(expansion.name.as_str(), &output_value)
          .unwrap_or_else(|| vec![output_value.clone()]);
        let declarations = vendor_values
          .iter()
          .map(|value| {
            if rule.important {
              format!("{}:{}!important", expansion.name, value)
            } else {
              format!("{}:{}", expansion.name, value)
            }
          })
          .collect::<Vec<_>>();
        let mut hash_component = hash_value.clone();
        if rule.important {
          hash_component.push_str("!important");
        }
        (declarations, hash_component)
      };
      let declaration = declaration_values.join(";");
      let value_hash = hash(&value_for_hash, 0);
      let value_segment = &value_hash[..value_hash.len().min(4)];

      let mut per_selector_outputs = Vec::new();
      for (selector_index, selector) in normalized_selectors.iter().enumerate() {
        let selectors_hash = selector.to_string();
        let group_hash = hash(
          &format!(
            "{}{}{}{}",
            prefix, at_rule_label, selectors_hash, expansion.name
          ),
          0,
        );
        let group = &group_hash[..group_hash.len().min(4)];
        if debug_hash {
          eprintln!(
            "[compiled-hash] group-input='{}{}{}{}' selector='{}' property='{}'",
            prefix, at_rule_label, selectors_hash, expansion.name, selector, expansion.name
          );
        }
        if debug_hash {
          eprintln!(
            "[compiled-hash] value-input='{}' important={}",
            value_for_hash, rule.important
          );
        }
        let full_class = format!("_{}{}", group, value_segment);
        let (class_name, selector_target) =
          match options.class_name_compression_map.get(&full_class[1..]) {
            Some(compressed) => (
              format!("_{}_{}", &full_class[1..5], compressed),
              compressed.clone(),
            ),
            None => (full_class.clone(), full_class.clone()),
          };
        let mut selector_output = join_selectors(&[selector.clone()], &selector_target);
        selector_output = selector_output
          .replace(" >[", ">[")
          .replace(" +[", "+[")
          .replace(" ~[", "~[");
        if let Some(original_selector) = rule.selectors.get(selector_index) {
          let trimmed_original = original_selector.trim_start();
          if trimmed_original.starts_with('[') {
            if let Some(close_index) = trimmed_original.find(']') {
              let rest = trimmed_original[close_index + 1..].trim_start();
              if rest.starts_with('+') || rest.starts_with('~') || rest.starts_with('>') {
                let pattern = format!(".{}[", selector_target);
                if selector_output.contains(&pattern) {
                  selector_output =
                    selector_output.replacen(&pattern, &format!(".{} [", selector_target), 1);
                }
              }
            }
          }
        }
        if options.increase_specificity {
          selector_output = apply_increase_specificity(&selector_output);
        }
        let css = wrap_at_rules(
          format!("{}{{{}}}", selector_output.clone(), declaration.clone()),
          &rule.at_rules,
        );
        per_selector_outputs.push((class_name, selector_output, css));
      }

      if !options.flatten_multiple_selectors && per_selector_outputs.len() > 1 {
        let mut selector_parts = per_selector_outputs
          .iter()
          .map(|(_, selector, _)| {
            let canonical = canonicalize_selector_key(selector.as_str());
            (canonical, selector.as_str())
          })
          .collect::<Vec<_>>();
        selector_parts.sort_by(|a, b| a.0.cmp(&b.0));
        let combined_selector = selector_parts
          .iter()
          .map(|(_, original)| *original)
          .collect::<Vec<_>>()
          .join(", ");
        let combined_css = wrap_at_rules(
          format!("{}{{{}}}", combined_selector, declaration.clone()),
          &rule.at_rules,
        );
        for (class_name, _, _) in per_selector_outputs {
          artifacts.push(AtomicRule {
            class_name,
            css: combined_css.clone(),
          });
        }
      } else {
        for (class_name, _, css) in per_selector_outputs {
          artifacts.push(AtomicRule { class_name, css });
        }
      }
    }
  }

  artifacts
}

/// A very small subset of the Babel CSS pipeline that focuses on atomicising flat
/// declarations. This is intentionally conservative but produces the same class
/// name hashing scheme as the JavaScript implementation so the outputs can be
/// compared in integration tests.
pub fn atomicize_literal(css: &str, options: &CssOptions) -> CssArtifacts {
  let mut artifacts = CssArtifacts::default();
  for declaration in css.split(';') {
    let trimmed = declaration.trim();
    if trimmed.is_empty() {
      continue;
    }
    if let Some((prop, value)) = trimmed.split_once(':') {
      let property = prop.trim();
      let mut raw_value = value.trim().to_string();
      let important_suffix;
      if let Some(stripped) = raw_value.strip_suffix("!important") {
        important_suffix = Some("!important");
        raw_value = stripped.trim_end().to_string();
      } else {
        important_suffix = None;
      }
      let NormalizedCssValue {
        hash_value,
        output_value,
      } = normalize_css_value(&raw_value);
      let prefix = options
        .class_hash_prefix
        .as_ref()
        .map(String::as_str)
        .unwrap_or("");
      let group_hash = hash(&format!("{}undefined{}", prefix, property), 0);
      let group = &group_hash[..group_hash.len().min(4)];
      let value_for_hash = match important_suffix {
        Some(flag) => format!("{}{}", hash_value, flag),
        None => hash_value.clone(),
      };
      let value_hash = hash(&value_for_hash, 0);
      let value_segment = &value_hash[..value_hash.len().min(4)];
      let full_class = format!("_{}{}", group, value_segment);
      let (class_name, selector_target) =
        match options.class_name_compression_map.get(&full_class[1..]) {
          Some(compressed) => (
            format!("_{}_{}", &full_class[1..5], compressed),
            compressed.clone(),
          ),
          None => (full_class.clone(), full_class.clone()),
        };
      let css_value = if let Some(flag) = important_suffix {
        format!("{}:{}{}", property, output_value, flag)
      } else {
        format!("{}:{}", property, output_value)
      };
      let mut selector = format!(".{}", selector_target);
      if options.increase_specificity {
        selector = apply_increase_specificity(&selector);
      }
      let css_rule = format!("{}{{{}}}", selector, css_value);
      artifacts.push(AtomicRule {
        class_name,
        css: css_rule,
      });
    }
  }
  artifacts
}

#[cfg(test)]
mod tests {
  use super::{
    CssOptions, CssRuleInput, atomicize_literal, atomicize_rules, minify_at_rule_params,
    minify_selector, normalize_css_value, normalize_selector, vendor_prefixed_values,
  };
  use crate::{extend_selectors, split_selector_list};

  #[test]
  fn generates_atomic_rules() {
    let artifacts = atomicize_literal("color: red; background: blue;", &CssOptions::default());
    assert_eq!(artifacts.rules.len(), 2);
    assert!(artifacts.class_names().any(|name| name.starts_with('_')));
  }

  #[test]
  fn compresses_class_names_when_map_present() {
    let mut options = CssOptions::default();
    options
      .class_name_compression_map
      .insert("1doq13q2".into(), "a".into());
    let artifacts = atomicize_literal("color: blue;", &options);
    assert_eq!(artifacts.rules.len(), 1);
    let rule = &artifacts.rules[0];
    assert_eq!(rule.class_name, "_1doq_a");
    assert!(rule.css.contains(".a{"));
  }

  #[test]
  fn converts_px_fallbacks_to_pc_inside_var() {
    let normalized = normalize_css_value("var(--ds-space-200, 16px)");
    assert_eq!(normalized.output_value, "var(--ds-space-200,1pc)");
    assert!(normalized.hash_value.contains("1pc"));
  }

  #[test]
  fn lowercases_hex_fallbacks_inside_var() {
    let normalized = normalize_css_value("var(--ds-surface-overlay, #FFFFFF)");
    assert_eq!(normalized.output_value, "var(--ds-surface-overlay,#fff)");
  }

  #[test]
  fn converts_rgba_to_hex() {
    let normalized = normalize_css_value("rgba(10, 20, 30, 0.8)");
    assert_eq!(normalized.output_value, "#0a141ecc");
  }

  #[test]
  fn converts_milliseconds_to_seconds() {
    let normalized = normalize_css_value("300ms");
    assert_eq!(normalized.output_value, ".3s");
    assert_eq!(normalized.hash_value, ".3s");
  }

  #[test]
  fn normalize_selector_preserves_combinator_space() {
    assert_eq!(normalize_selector(Some("> button")), "&>button".to_string());
    assert_eq!(normalize_selector(Some(">button")), "&>button".to_string());
    assert_eq!(normalize_selector(Some(" >button")), "&>button".to_string());
    assert_eq!(minify_selector("& >button"), "&>button".to_string());
    assert_eq!(
      normalize_selector(Some("& >button")),
      "&>button".to_string()
    );
    assert_eq!(
      normalize_selector(Some("& > [data-ds--text-field--input]")),
      "&>[data-ds--text-field--input]".to_string()
    );
    assert_eq!(
      normalize_selector(Some("> :is(div,button)")),
      "& >:is(div,button)".to_string()
    );
    assert_eq!(normalize_selector(Some("> *")), "& >*".to_string());
  }

  #[test]
  fn normalize_selector_normalizes_attribute_quotes() {
    assert_eq!(
      normalize_selector(Some("& [data-testid='example.value']")),
      "& [data-testid=\"example.value\"]".to_string()
    );
  }

  #[test]
  fn split_selector_preserves_is_arguments() {
    let segments = split_selector_list(">:is(div, button)");
    assert_eq!(segments, vec![">:is(div, button)"]);
    let segments_with_space = split_selector_list(" >:is(div, button)");
    assert_eq!(segments_with_space, vec![" >:is(div, button)"]);
  }

  #[test]
  fn extend_selectors_preserves_is_arguments() {
    let parents = vec!["&".to_string()];
    let extended = extend_selectors(&parents, ">:is(div, button)");
    assert_eq!(extended, vec!["& >:is(div,button)".to_string()]);
  }

  #[test]
  fn normalize_selector_removes_unnecessary_attribute_quotes() {
    assert_eq!(
      normalize_selector(Some("[data-control=\"set-default\"] button")),
      "&[data-control=set-default] button".to_string()
    );
  }

  #[test]
  fn does_not_convert_px_inside_identifiers() {
    let normalized = normalize_css_value("url(/images/icon-16px.png)");
    assert_eq!(normalized.output_value, "url(/images/icon-16px.png)");
  }

  #[test]
  fn strips_zero_units_inside_var() {
    let normalized = normalize_css_value("var(--ds-space-0, 0px)");
    assert_eq!(normalized.output_value, "var(--ds-space-0,0)");
  }

  #[test]
  fn preserves_zero_unit_in_custom_property() {
    let rule = CssRuleInput {
      selectors: vec!["&".into()],
      at_rules: vec![],
      property: "--foo".into(),
      value: "0px".into(),
      raw_value: "0px".into(),
      important: false,
    };
    let artifacts = atomicize_rules(&[rule], &CssOptions::default());
    let css_rule = &artifacts.rules[0].css;
    assert!(css_rule.contains("--foo:0px"));
  }

  #[test]
  fn strips_trailing_decimal_zeroes_in_numbers() {
    let normalized = normalize_css_value("cubic-bezier(0.15, 1.0, 0.3, 1.0)");
    assert_eq!(
      normalized.hash_value.replace(' ', ""),
      "cubic-bezier(.15,1,.3,1)"
    );
    assert!(normalized.output_value.contains("cubic-bezier(.15,1,.3,1)"));
  }

  #[test]
  fn preserves_decimal_in_identifiers() {
    let normalized = normalize_css_value("var(--ds-token-1.0)");
    assert_eq!(normalized.output_value, "var(--ds-token-1.0)");
  }

  #[test]
  fn vendor_prefixed_values_for_fit_content() {
    let values = vendor_prefixed_values("width", "fit-content").expect("expected expansion");
    assert_eq!(values, vec!["-moz-fit-content", "fit-content"]);
    assert!(vendor_prefixed_values("width", "-moz-fit-content").is_none());
  }

  #[test]
  fn atomicize_rules_emits_vendor_prefixed_fit_content() {
    let rule = CssRuleInput {
      selectors: vec!["&".to_string()],
      at_rules: vec![],
      property: "width".into(),
      value: "fit-content".into(),
      raw_value: "fit-content".into(),
      important: false,
    };
    let artifacts = atomicize_rules(&[rule], &CssOptions::default());
    let css_rule = &artifacts.rules[0].css;
    assert!(
      css_rule.contains("width:-moz-fit-content;width:fit-content"),
      "css rule was {css_rule}"
    );
  }

  #[test]
  fn atomicize_rules_expands_overflow_shorthand() {
    let rule = CssRuleInput {
      selectors: vec!["&".to_string()],
      at_rules: vec![],
      property: "overflow".into(),
      value: "hidden".into(),
      raw_value: "hidden".into(),
      important: false,
    };
    let artifacts = atomicize_rules(&[rule], &CssOptions::default());
    let css_strings: Vec<&str> = artifacts
      .rules
      .iter()
      .map(|rule| rule.css.as_str())
      .collect();
    assert!(
      css_strings
        .iter()
        .any(|css| css.contains("overflow-x:hidden")),
      "css strings were {:?}",
      css_strings
    );
    assert!(
      css_strings
        .iter()
        .any(|css| css.contains("overflow-y:hidden")),
      "css strings were {:?}",
      css_strings
    );
  }

  #[test]
  fn atomicize_rules_expands_flex_shorthand_one() {
    let rule = CssRuleInput {
      selectors: vec!["&".to_string()],
      at_rules: vec![],
      property: "flex".into(),
      value: "1".into(),
      raw_value: "1".into(),
      important: false,
    };
    let artifacts = atomicize_rules(&[rule], &CssOptions::default());
    let css_strings: Vec<&str> = artifacts
      .rules
      .iter()
      .map(|rule| rule.css.as_str())
      .collect();
    assert!(
      css_strings.iter().any(|css| css.contains("flex-grow:1")),
      "css strings were {:?}",
      css_strings
    );
    assert!(
      css_strings.iter().any(|css| css.contains("flex-shrink:1")),
      "css strings were {:?}",
      css_strings
    );
    assert!(
      css_strings.iter().any(|css| css.contains("flex-basis:0%")),
      "css strings were {:?}",
      css_strings
    );
  }

  #[test]
  fn minifies_media_query_parameters() {
    assert_eq!(
      minify_at_rule_params("(min-width: 90rem)"),
      "(min-width:90rem)"
    );
    assert_eq!(
      minify_at_rule_params("screen and (min-width: 90rem)"),
      "screen and (min-width:90rem)"
    );
  }

  #[test]
  fn normalize_selector_collapses_pseudo_element_colons() {
    assert_eq!(normalize_selector(Some("span::before")), "& span:before");
    assert_eq!(normalize_selector(Some("::after")), "&:after");
    assert_eq!(normalize_selector(Some("&::before")), "&:before");
    assert_eq!(
      normalize_selector(Some("[data-attr=\"a::b\"]")),
      "&[data-attr=\"a::b\"]"
    );
  }
}
