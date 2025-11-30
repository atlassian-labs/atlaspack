use std::sync::Arc;

use once_cell::sync::Lazy;
use oxc_browserslist::{execute, Opts};
use regex::Regex;
use swc_core::common::{input::StringInput, FileName, SourceMap};
use swc_core::css::ast::{
  AtRule, AtRuleName, AtRulePrelude, ComponentValue, QualifiedRule, Rule, SimpleBlock, Stylesheet,
};
use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

use super::super::transform::{Plugin, TransformContext};

static ASPECT_RATIO_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"(?i)(-aspect-ratio\s*:\s*)(\d+)\s*/\s*(\d+)")
    .expect("failed to compile aspect ratio regex")
});

#[derive(Debug, Clone)]
pub struct MinifyParams {
  has_all_bug: bool,
}

pub fn minify_params() -> MinifyParams {
  MinifyParams::new()
}

impl MinifyParams {
  fn new() -> Self {
    Self {
      has_all_bug: detect_all_bug(),
    }
  }

  #[cfg(test)]
  fn with_has_all_bug(has_all_bug: bool) -> Self {
    Self { has_all_bug }
  }

  fn process_stylesheet(&self, stylesheet: &mut Stylesheet) {
    for rule in &mut stylesheet.rules {
      self.process_rule(rule);
    }
  }

  fn process_rule(&self, rule: &mut Rule) {
    match rule {
      Rule::AtRule(at_rule) => {
        self.process_at_rule(at_rule);
        if let Some(block) = &mut at_rule.block {
          self.process_simple_block(block);
        }
      }
      Rule::QualifiedRule(rule) => {
        self.process_qualified_rule(rule);
      }
      Rule::ListOfComponentValues(list) => {
        self.process_component_values(&mut list.children);
      }
    }
  }

  fn process_qualified_rule(&self, rule: &mut QualifiedRule) {
    self.process_simple_block(&mut rule.block);
  }

  fn process_simple_block(&self, block: &mut SimpleBlock) {
    self.process_component_values(&mut block.value);
  }

  fn process_component_values(&self, values: &mut [ComponentValue]) {
    for value in values {
      match value {
        ComponentValue::AtRule(at_rule) => {
          self.process_at_rule(at_rule);
          if let Some(block) = &mut at_rule.block {
            self.process_simple_block(block);
          }
        }
        ComponentValue::QualifiedRule(rule) => {
          self.process_qualified_rule(rule);
        }
        ComponentValue::SimpleBlock(block) => {
          self.process_simple_block(block);
        }
        ComponentValue::ListOfComponentValues(list) => {
          self.process_component_values(&mut list.children);
        }
        ComponentValue::Function(function) => {
          self.process_component_values(&mut function.value);
        }
        ComponentValue::KeyframeBlock(block) => {
          self.process_simple_block(&mut block.block);
        }
        _ => {}
      }
    }
  }

  fn process_at_rule(&self, at_rule: &mut AtRule) {
    let name = at_rule_name(&at_rule.name);
    if name != "media" && name != "supports" {
      return;
    }

    let Some(prelude) = at_rule.prelude.take() else {
      return;
    };

    let serialized = serialize_prelude(&prelude);
    if serialized.trim().is_empty() {
      at_rule.prelude = None;
      return;
    }

    let minified = minify_params_string(&serialized, name == "media", self.has_all_bug);
    if minified.is_empty() {
      at_rule.prelude = None;
      return;
    }

    match parse_prelude(&name, &minified) {
      Some(parsed) => at_rule.prelude = Some(parsed),
      None => at_rule.prelude = Some(prelude),
    }
  }
}

impl Plugin for MinifyParams {
  fn name(&self) -> &'static str {
    "postcss-minify-params"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    self.process_stylesheet(stylesheet);
  }
}

fn detect_all_bug() -> bool {
  let mut opts = Opts::default();
  opts.path = Some(env!("CARGO_MANIFEST_DIR").to_string());

  execute(&opts)
    .map(|list| {
      list.into_iter().any(|entry| {
        let name = entry.name().to_ascii_lowercase();
        let version = entry.version().to_ascii_lowercase();
        name == "ie" && (version == "10" || version == "11")
      })
    })
    .unwrap_or(false)
}

fn minify_params_string(input: &str, is_media: bool, has_all_bug: bool) -> String {
  let mut normalized = normalize_whitespace(input);
  reduce_aspect_ratios(&mut normalized);

  if is_media && !has_all_bug {
    remove_leading_all(&mut normalized);
  }

  ensure_keyword_spacing(&mut normalized);

  let mut segments = split_arguments(&normalized);
  segments.sort();
  segments.dedup();
  let mut result = segments.join(",");
  if result.starts_with('(') {
    result.insert(0, ' ');
  }
  result
}

fn normalize_whitespace(input: &str) -> String {
  let mut output = String::with_capacity(input.len());
  let mut i = 0usize;
  let mut depth = 0usize;
  let mut in_string: Option<char> = None;

  while i < input.len() {
    let ch = input[i..].chars().next().unwrap();
    let ch_len = ch.len_utf8();

    if let Some(quote) = in_string {
      output.push_str(&input[i..i + ch_len]);
      i += ch_len;

      if ch == '\\' {
        if i < input.len() {
          let next = input[i..].chars().next().unwrap();
          let next_len = next.len_utf8();
          output.push_str(&input[i..i + next_len]);
          i += next_len;
        }
      } else if ch == quote {
        in_string = None;
      }

      continue;
    }

    match ch {
      '\'' | '"' => {
        in_string = Some(ch);
        output.push_str(&input[i..i + ch_len]);
        i += ch_len;
      }
      '(' => {
        depth += 1;
        while output.ends_with(' ') {
          output.pop();
        }
        let needs_space = needs_space_before_paren(&output);
        if needs_space && !output.ends_with(' ') {
          output.push(' ');
        }
        output.push('(');
        i += ch_len;
        while i < input.len() {
          let next = input[i..].chars().next().unwrap();
          if next.is_whitespace() {
            i += next.len_utf8();
          } else {
            break;
          }
        }
      }
      ')' => {
        if depth > 0 {
          depth -= 1;
        }
        while output.ends_with(' ') {
          output.pop();
        }
        output.push(')');
        i += ch_len;
      }
      ':' | ',' | '/' => {
        while output.ends_with(' ') {
          output.pop();
        }
        output.push(ch);
        i += ch_len;
        while i < input.len() {
          let next = input[i..].chars().next().unwrap();
          if next.is_whitespace() {
            i += next.len_utf8();
          } else {
            break;
          }
        }
      }
      _ if ch.is_whitespace() => {
        if !output.is_empty() && !output.ends_with(' ') {
          output.push(' ');
        }
        i += ch_len;
        while i < input.len() {
          let next = input[i..].chars().next().unwrap();
          if next.is_whitespace() {
            i += next.len_utf8();
          } else {
            break;
          }
        }
      }
      _ => {
        output.push_str(&input[i..i + ch_len]);
        i += ch_len;
      }
    }
  }

  while output.ends_with(' ') {
    output.pop();
  }

  output
}

fn reduce_aspect_ratios(value: &mut String) {
  let mut result = String::with_capacity(value.len());
  let mut last = 0;

  for capture in ASPECT_RATIO_RE.captures_iter(value) {
    let full = capture.get(0).unwrap();
    let prefix = capture.get(1).unwrap().as_str();
    let a = capture.get(2).unwrap().as_str().parse::<u32>().unwrap_or(0);
    let b = capture.get(3).unwrap().as_str().parse::<u32>().unwrap_or(0);
    let divisor = gcd(a, b);
    let left = if divisor == 0 { a } else { a / divisor };
    let right = if divisor == 0 { b } else { b / divisor };

    result.push_str(&value[last..full.start()]);
    result.push_str(prefix);
    result.push_str(&left.to_string());
    result.push('/');
    result.push_str(&right.to_string());
    last = full.end();
  }

  result.push_str(&value[last..]);
  *value = result;
}

fn remove_leading_all(value: &mut String) {
  let trimmed = value.trim_start();
  let offset = value.len() - trimmed.len();
  let lower = trimmed.to_ascii_lowercase();

  if !lower.starts_with("all") {
    return;
  }

  let after_all = &trimmed[3..];
  if !after_all.is_empty()
    && !after_all.starts_with(' ')
    && !after_all.starts_with('(')
    && !after_all.starts_with(',')
  {
    return;
  }

  let mut remainder = after_all.trim_start().to_string();
  let lowercase = remainder.to_ascii_lowercase();
  if lowercase.starts_with("and") {
    let rest = &remainder[3..];
    if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('(') || rest.starts_with(',') {
      remainder = rest.trim_start().to_string();
    }
  }

  value.replace_range(.., &format!("{}{}", &value[..offset], remainder));
}

fn split_arguments(input: &str) -> Vec<String> {
  let mut args = Vec::new();
  let mut start = 0usize;
  let mut depth = 0usize;
  let mut in_string: Option<char> = None;
  let mut i = 0usize;

  while i < input.len() {
    let ch = input[i..].chars().next().unwrap();
    let ch_len = ch.len_utf8();

    if let Some(quote) = in_string {
      if ch == '\\' {
        i += ch_len;
        if i < input.len() {
          let next = input[i..].chars().next().unwrap();
          i += next.len_utf8();
        }
        continue;
      }

      if ch == quote {
        in_string = None;
      }

      i += ch_len;
      continue;
    }

    match ch {
      '\'' | '"' => {
        in_string = Some(ch);
      }
      '(' => {
        depth += 1;
      }
      ')' => {
        if depth > 0 {
          depth -= 1;
        }
      }
      ',' if depth == 0 => {
        let segment = input[start..i].trim();
        if !segment.is_empty() {
          args.push(segment.to_string());
        }
        start = i + ch_len;
      }
      _ => {}
    }

    i += ch_len;
  }

  let segment = input[start..].trim();
  if !segment.is_empty() {
    args.push(segment.to_string());
  }

  args
}

fn ensure_keyword_spacing(value: &mut String) {
  const KEYWORDS: [&str; 3] = ["and", "or", "not"];
  for keyword in KEYWORDS {
    let suffix_pattern = format!("){keyword}");
    let suffix_replacement = format!(") {keyword}");
    while let Some(index) = value.find(&suffix_pattern) {
      value.replace_range(index..index + suffix_pattern.len(), &suffix_replacement);
    }

    let prefix_pattern = format!("{keyword}(");
    let prefix_replacement = format!("{keyword} (");
    while let Some(index) = value.find(&prefix_pattern) {
      value.replace_range(index..index + prefix_pattern.len(), &prefix_replacement);
    }
  }
}

fn needs_space_before_paren(output: &str) -> bool {
  fn ends_with_ident(target: &str, ident: &str) -> bool {
    if !target.ends_with(ident) {
      return false;
    }

    let prefix = &target[..target.len() - ident.len()];
    match prefix.chars().last() {
      Some(ch) if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' => false,
      _ => true,
    }
  }

  const KEYWORDS: [&str; 6] = ["and", "or", "not", "only", "@supports", "@media"];
  KEYWORDS.iter().any(|kw| ends_with_ident(output, kw))
}

fn at_rule_name(name: &AtRuleName) -> String {
  match name {
    AtRuleName::Ident(ident) => ident.value.to_lowercase(),
    AtRuleName::DashedIdent(ident) => ident.value.to_lowercase(),
  }
}

fn serialize_prelude(prelude: &AtRulePrelude) -> String {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
    generator
      .emit(prelude)
      .expect("failed to serialize at-rule prelude");
  }
  output
}

fn parse_prelude(name: &str, params: &str) -> Option<Box<AtRulePrelude>> {
  let snippet = format!("@{} {}{{}}", name, params);
  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom("params.css".into()).into(), snippet);
  let mut errors = vec![];

  let stylesheet = parse_string_input::<Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  )
  .ok()?;

  if !errors.is_empty() {
    return None;
  }

  stylesheet.rules.into_iter().find_map(|rule| match rule {
    Rule::AtRule(rule) => rule.prelude,
    _ => None,
  })
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
  if a == 0 {
    return b;
  }
  if b == 0 {
    return a;
  }
  while b != 0 {
    let temp = b;
    b = a % b;
    a = temp;
  }
  a
}

impl Default for MinifyParams {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};

  fn transform(css: &str) -> Stylesheet {
    let mut stylesheet = parse_stylesheet(css);
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    MinifyParams::with_has_all_bug(false).run(&mut stylesheet, &mut ctx);
    stylesheet
  }

  fn parse_stylesheet(css: &str) -> Stylesheet {
    let cm: Arc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.css".into()).into(), css.into());
    let mut errors = vec![];
    parse_string_input::<Stylesheet>(
      StringInput::from(&*fm),
      None,
      ParserConfig::default(),
      &mut errors,
    )
    .expect("failed to parse test stylesheet")
  }

  #[test]
  fn removes_all_keyword() {
    let css = "@media all and (min-width: 500px) {a{color:red}}";
    let stylesheet = transform(css);
    assert_eq!(first_prelude(&stylesheet), "(min-width: 500px)");
  }

  #[test]
  fn normalizes_aspect_ratio() {
    let css = "@media (min-aspect-ratio: 18/12) {a{color:red}}";
    let stylesheet = transform(css);
    assert_eq!(first_prelude(&stylesheet), "(min-aspect-ratio: 3/2)");
  }

  #[test]
  fn dedupes_and_sorts_queries() {
    let css = "@media tv and (min-width:700px), screen and (max-width:600px), screen and (max-width:600px) {a{color:red}}";
    let stylesheet = transform(css);
    assert_eq!(
      first_prelude(&stylesheet),
      "screen and (max-width: 600px), tv and (min-width: 700px)"
    );
  }

  #[test]
  fn preserves_supports_spacing() {
    let css = "@supports (--foo: red) and (display: flex) {a{color:red}}";
    let stylesheet = transform(css);
    assert_eq!(
      first_prelude(&stylesheet),
      "(--foo: red) and (display: flex)"
    );
  }

  fn first_prelude(stylesheet: &Stylesheet) -> String {
    let prelude = stylesheet
      .rules
      .iter()
      .find_map(|rule| match rule {
        Rule::AtRule(at_rule) => at_rule.prelude.as_deref(),
        _ => None,
      })
      .expect("expected stylesheet to contain an at-rule");
    serialize_prelude_pretty(prelude).trim_start().to_string()
  }

  fn serialize_prelude_pretty(prelude: &AtRulePrelude) -> String {
    let mut output = String::new();
    {
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
      generator
        .emit(prelude)
        .expect("failed to serialize at-rule prelude");
    }
    output
  }
}
