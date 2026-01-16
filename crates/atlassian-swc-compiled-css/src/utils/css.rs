use std::borrow::Cow;
use std::collections::HashSet;

use once_cell::sync::Lazy;
use regex::Regex;

/// Represents the CSS preceding an interpolation when extracting affixes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeforeInterpolation {
  pub css: String,
  pub variable_prefix: String,
}

/// Represents the CSS following an interpolation when extracting affixes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfterInterpolation {
  pub css: String,
  pub variable_suffix: String,
}

/// Runtime value supported by `add_unit_if_needed`.
#[derive(Clone, Debug, PartialEq)]
pub enum CssValue<'a> {
  Null,
  Bool(bool),
  String(Cow<'a, str>),
  Number(f64),
}

const UPPER_A_GRAVE: char = '\u{00C0}';
const UPPER_O_DIAERESIS: char = '\u{00D6}';
const UPPER_O_SLASH: char = '\u{00D8}';
const UPPER_THORN: char = '\u{00DE}';

static UNIT_REGEX: Lazy<Regex> = Lazy::new(|| {
  const UNITS: &[&str] = &[
    "em", "ex", "cap", "ch", "ic", "rem", "lh", "rlh", "vw", "vh", "vi", "vb", "vmin", "vmax",
    "cm", "mm", "Q", "in", "pc", "pt", "px", "deg", "grad", "rad", "turn", "s", "ms", "Hz", "kHz",
    "dpi", "dpcm", "dppx", "x", "fr", "%",
  ];

  let pattern = format!(
    "^(({}|\"|'))(;|,|\\n| |\\\\)?",
    UNITS
      .iter()
      .map(|unit| regex::escape(unit))
      .collect::<Vec<_>>()
      .join("|")
  );

  Regex::new(&pattern).expect("valid css unit regex")
});

static UNITLESS_PROPERTIES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
  HashSet::from([
    "animationIterationCount",
    "basePalette",
    "borderImageOutset",
    "borderImageSlice",
    "borderImageWidth",
    "boxFlex",
    "boxFlexGroup",
    "boxOrdinalGroup",
    "columnCount",
    "columns",
    "flex",
    "flexGrow",
    "flexPositive",
    "flexShrink",
    "flexNegative",
    "flexOrder",
    "fontSizeAdjust",
    "fontWeight",
    "gridArea",
    "gridRow",
    "gridRowEnd",
    "gridRowSpan",
    "gridRowStart",
    "gridColumn",
    "gridColumnEnd",
    "gridColumnSpan",
    "gridColumnStart",
    "lineClamp",
    "lineHeight",
    "opacity",
    "order",
    "orphans",
    "tabSize",
    "WebkitLineClamp",
    "widows",
    "zIndex",
    "zoom",
    "fillOpacity",
    "floodOpacity",
    "stopOpacity",
    "strokeDasharray",
    "strokeDashoffset",
    "strokeMiterlimit",
    "strokeOpacity",
    "strokeWidth",
  ])
});

fn css_before_interpolation(input: &str) -> BeforeInterpolation {
  let mut css = input.to_string();
  let mut variable_prefix = String::new();

  if let Some(last) = css.chars().last() {
    if matches!(last, '"' | '\'' | '-') {
      css.pop();
      variable_prefix.push(last);
    }
  }

  BeforeInterpolation {
    css,
    variable_prefix,
  }
}

fn css_after_interpolation(input: &str) -> AfterInterpolation {
  if let Some(captures) = UNIT_REGEX.captures(input) {
    if let Some(unit) = captures.get(1) {
      let mut css = input.to_string();
      css.replace_range(unit.range(), "");
      return AfterInterpolation {
        css,
        variable_suffix: unit.as_str().to_string(),
      };
    }
  }

  AfterInterpolation {
    css: input.to_string(),
    variable_suffix: String::new(),
  }
}

/// Extracts prefix and suffix around an interpolation, mirroring the behaviour of the JS helper.
pub fn css_affix_interpolation(
  before: &str,
  after: &str,
) -> (BeforeInterpolation, AfterInterpolation) {
  if before.ends_with("url(") && after.starts_with(')') {
    let mut css_before = before.to_string();
    for _ in 0.."url(".len() {
      css_before.pop();
    }

    let mut css_after = after.to_string();
    css_after.remove(0);

    return (
      BeforeInterpolation {
        css: css_before,
        variable_prefix: "url(".into(),
      },
      AfterInterpolation {
        css: css_after,
        variable_suffix: ")".into(),
      },
    );
  }

  (
    css_before_interpolation(before),
    css_after_interpolation(after),
  )
}

/// Converts camelCase strings into kebab-case, matching the JS implementation.
pub fn kebab_case(input: &str) -> String {
  let mut result = String::with_capacity(input.len());

  for ch in input.chars() {
    let is_upper = matches!(
        ch,
        'A'..='Z'
            | UPPER_A_GRAVE..=UPPER_O_DIAERESIS
            | UPPER_O_SLASH..=UPPER_THORN
    );

    if is_upper {
      result.push('-');
      for lower in ch.to_lowercase() {
        result.push(lower);
      }
    } else {
      result.push(ch);
    }
  }

  result
}

/// Mirrors the behaviour of `addUnitIfNeeded` from `@compiled/css`.
fn format_css_number(mut num: f64) -> String {
  // Normalize -0 to 0 to avoid "-0" outputs.
  if num == -0.0 {
    num = 0.0;
  }
  let mut string = num.to_string();
  if string.ends_with(".0") {
    string.truncate(string.len() - 2);
  }
  if string.starts_with("0.") {
    string.remove(0);
  } else if string.starts_with("-0.") {
    string.remove(1);
  }
  string
}

pub fn add_unit_if_needed(property: &str, value: CssValue<'_>) -> String {
  match value {
    CssValue::Null => String::new(),
    CssValue::Bool(_) => String::new(),
    CssValue::String(text) => text.trim().to_string(),
    CssValue::Number(num) => {
      if num == 0.0 || UNITLESS_PROPERTIES.contains(property) {
        format_css_number(num)
      } else {
        format!("{}px", format_css_number(num))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{
    AfterInterpolation, BeforeInterpolation, CssValue, add_unit_if_needed, css_affix_interpolation,
    kebab_case,
  };

  #[test]
  fn kebab_case_converts_uppercase() {
    assert_eq!(kebab_case("backgroundColor"), "background-color");
    assert_eq!(kebab_case("Öresund"), "-öresund");
  }

  #[test]
  fn affix_interpolation_handles_quotes_and_units() {
    let (before, after) = css_affix_interpolation("content: \"", "\";color: blue;");
    assert_eq!(
      before,
      BeforeInterpolation {
        css: "content: ".into(),
        variable_prefix: "\"".into(),
      }
    );
    assert_eq!(
      after,
      AfterInterpolation {
        css: ";color: blue;".into(),
        variable_suffix: "\"".into(),
      }
    );

    let (before, after) = css_affix_interpolation("padding: 0 ", "px 0");
    assert_eq!(before.variable_prefix, "");
    assert_eq!(after.variable_suffix, "px");
  }

  #[test]
  fn affix_interpolation_handles_url_function() {
    // When url( and ) surround an interpolation, they should be extracted
    // as prefix/suffix so they can be applied to the runtime value, not the var() itself
    let (before, after) = css_affix_interpolation("background-image: url(", ")");
    assert_eq!(before.variable_prefix, "url(");
    assert_eq!(after.variable_suffix, ")");
    assert_eq!(before.css, "background-image: ");
    assert_eq!(after.css, "");
  }

  #[test]
  fn affix_interpolation_preserves_url_with_additional_css() {
    // url( and ) should be extracted even when there's additional CSS after
    let (before, after) = css_affix_interpolation("background-image: url(", "); color: red;");
    assert_eq!(before.variable_prefix, "url(");
    assert_eq!(after.variable_suffix, ")");
    assert_eq!(before.css, "background-image: ");
    assert_eq!(after.css, "; color: red;");
  }

  #[test]
  fn affix_interpolation_handles_non_url_cases() {
    // Quotes should also be extracted as prefix/suffix
    let (before, after) = css_affix_interpolation("content: \"", "\"; color: blue;");
    assert_eq!(before.variable_prefix, "\"");
    assert_eq!(after.variable_suffix, "\"");
    assert_eq!(before.css, "content: ");
    assert_eq!(after.css, "; color: blue;");
  }

  #[test]
  fn affix_interpolation_does_not_extract_url_without_closing_paren() {
    // If `before` ends with "url(" but `after` doesn't start with ")",
    // it should not extract the url( as a prefix
    let (before, after) = css_affix_interpolation("background: url(", "; color: red;");
    assert_eq!(before.variable_prefix, "");
    assert_eq!(after.variable_suffix, "");
    assert_eq!(before.css, "background: url(");
    assert_eq!(after.css, "; color: red;");
  }

  #[test]
  fn affix_interpolation_handles_units() {
    // Units like px should be extracted as suffix
    let (before, after) = css_affix_interpolation("padding: ", "px 10px");
    assert_eq!(before.variable_prefix, "");
    assert_eq!(after.variable_suffix, "px");
    assert_eq!(before.css, "padding: ");
    assert_eq!(after.css, " 10px");
  }

  #[test]
  fn add_unit_skips_unitless_properties() {
    assert_eq!(add_unit_if_needed("opacity", CssValue::Number(0.5)), ".5");
    assert_eq!(add_unit_if_needed("opacity", CssValue::Number(-0.5)), "-.5");
  }

  #[test]
  fn add_unit_appends_px_for_non_unitless() {
    assert_eq!(add_unit_if_needed("margin", CssValue::Number(4.0)), "4px");
  }

  #[test]
  fn add_unit_returns_trimmed_string() {
    assert_eq!(
      add_unit_if_needed("color", CssValue::String("  blue  ".into())),
      "blue"
    );
  }

  #[test]
  fn add_unit_returns_empty_for_nullish() {
    assert_eq!(add_unit_if_needed("color", CssValue::Null), "");
    assert_eq!(add_unit_if_needed("color", CssValue::Bool(true)), "");
  }
}
