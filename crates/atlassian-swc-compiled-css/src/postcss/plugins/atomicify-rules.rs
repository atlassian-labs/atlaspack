use std::borrow::Cow;
use std::sync::Arc;

use swc_core::common::{FileName, SourceMap, Spanned, input::StringInput};
use swc_core::css::ast::{
  AtRule, AtRuleName, AtRulePrelude, ComponentValue, Declaration, DeclarationName,
  ListOfComponentValues, QualifiedRule, QualifiedRulePrelude, Rule, Stylesheet,
};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

use super::super::transform::{Plugin, TransformContext};
use crate::utils_hash::hash;

#[derive(Debug, Default, Clone, Copy)]
pub struct AtomicifyRules;

impl Plugin for AtomicifyRules {
  fn name(&self) -> &'static str {
    "atomicify-rules"
  }

  fn run(&self, stylesheet: &mut Stylesheet, ctx: &mut TransformContext<'_>) {
    if let Some(prefix) = ctx.options.class_hash_prefix.as_deref() {
      if !is_css_identifier_valid(prefix) {
        panic!(
          "{} isn't a valid CSS identifier. Accepted characters are ^[a-zA-Z\\-_]+[a-zA-Z\\-_0-9]*$",
          prefix
        );
      }
    }

    let options = AtomicifyOptions {
      class_name_compression_map: ctx.options.class_name_compression_map.as_ref(),
      class_hash_prefix: ctx.options.class_hash_prefix.as_deref(),
      declaration_placeholder: ctx.options.declaration_placeholder.as_deref(),
    };

    let mut transformed: Vec<Rule> = Vec::with_capacity(stylesheet.rules.len());

    for rule in std::mem::take(&mut stylesheet.rules) {
      match rule {
        Rule::AtRule(at_rule) => {
          if can_atomicify_at_rule(&at_rule) {
            let converted = atomicify_at_rule(*at_rule, &options, ctx, "");
            transformed.push(Rule::AtRule(Box::new(converted)));
          } else {
            transformed.push(Rule::AtRule(at_rule));
          }
        }
        Rule::QualifiedRule(rule) => {
          let sels = collect_rule_selectors(&rule);
          println!("[atomicify.rule.pre] selectors={:?}", sels);
          let replacements = atomicify_qualified_rule(*rule, &options, ctx, None);
          for replacement in replacements {
            transformed.push(Rule::QualifiedRule(Box::new(replacement)));
          }
        }
        Rule::ListOfComponentValues(list) => {
          if !is_comment_list(&list) {
            transformed.push(Rule::ListOfComponentValues(list));
          }
        }
      }
    }

    stylesheet.rules = transformed;
  }
}

pub fn atomicify_rules() -> AtomicifyRules {
  AtomicifyRules
}

struct AtomicifyOptions<'a> {
  class_name_compression_map: Option<&'a std::collections::HashMap<String, String>>,
  class_hash_prefix: Option<&'a str>,
  declaration_placeholder: Option<&'a str>,
}

fn normalize_selectors(selectors: Vec<String>, options: &AtomicifyOptions<'_>) -> Vec<String> {
  if let Some(placeholder) = options.declaration_placeholder {
    selectors
      .into_iter()
      .map(|selector| {
        let trimmed = selector.trim();

        if trimmed == placeholder {
          String::new()
        } else if trimmed.contains(placeholder) {
          trimmed.replace(placeholder, "").trim().to_string()
        } else {
          selector
        }
      })
      .collect()
  } else {
    selectors
  }
}

fn atomicify_qualified_rule(
  rule: QualifiedRule,
  options: &AtomicifyOptions<'_>,
  ctx: &mut TransformContext<'_>,
  at_rule_label: Option<&str>,
) -> Vec<QualifiedRule> {
  let selectors =
    filter_redundant_selectors(normalize_selectors(collect_rule_selectors(&rule), options))
      .into_iter()
      .map(|sel| {
        let mut cleaned = sel.replace(" &", "&").replace("& ", "&");
        // COMPAT: collapse descendant gap between repeated class in orphaned pseudo chains
        // e.g. "._a:focus ._a:before" or "._a:focus-within ._a:before" -> "._a:focus._a:before"
        if let Some((left, right)) = cleaned.split_once(' ') {
          if left.starts_with("._") && right.starts_with('.') {
            let left_parts: Vec<&str> = left.split('.').collect();
            let right_parts: Vec<&str> = right.split('.').collect();
            if left_parts.get(1) == right_parts.get(1) {
              cleaned = format!("{}{}", left, right);
            }
          }
        }
        // Regex-ish collapse for descendant between identical class hashes followed by pseudos
        // to mirror Babel combined pseudo selectors.
        if let Some(idx) = cleaned.find(" ._") {
          let (l, r) = cleaned.split_at(idx);
          let right = &r[1..]; // drop leading space
          if l.starts_with("._") && right.starts_with("._") {
            let lhash = l.split(':').next().unwrap_or(l);
            let rhash = right.split(':').next().unwrap_or(right);
            if lhash == rhash {
              cleaned = format!("{}{}", l, right);
            }
          }
        }
        // Final fallback: if cleaned still has ' ._' and both sides share the same hash, strip the space.
        if let Some(idx) = cleaned.find(" ._") {
          let (l, r) = cleaned.split_at(idx);
          let right = &r[1..];
          if l.starts_with("._") && right.starts_with("._") {
            let lhash = l.split(':').next().unwrap_or(l);
            let rhash = right.split(':').next().unwrap_or(right);
            if lhash == rhash {
              cleaned = format!("{}{}", l, right);
            }
          }
        }
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!(
            "[atomicify.rule] selector raw='{}' cleaned='{}'",
            sel, cleaned
          );
        }
        cleaned
      })
      .filter(|sel| !sel.contains("* *"))
      .collect::<Vec<String>>();
  let mut replacements: Vec<QualifiedRule> = Vec::new();

  for component in rule.block.value {
    match component {
      ComponentValue::Declaration(declaration) => {
        let declaration = *declaration;
        let atomic_rule =
          atomicify_declaration(&declaration, &selectors, options, ctx, at_rule_label);
        replacements.push(atomic_rule);
      }
      ComponentValue::QualifiedRule(_) => {
        panic!(
          "atomicify-rules: Nested rules need to be flattened first - run the \"postcss-nested\" plugin before this."
        );
      }
      _ => {}
    }
  }

  replacements
}

fn atomicify_at_rule(
  mut at_rule: AtRule,
  options: &AtomicifyOptions<'_>,
  ctx: &mut TransformContext<'_>,
  parent_label: &str,
) -> AtRule {
  let name = at_rule_name(&at_rule.name);
  let params = at_rule_params(&at_rule);
  let label = format!("{}{}{}", parent_label, name, params);

  let Some(mut block) = at_rule.block.take() else {
    return at_rule;
  };

  let mut new_children: Vec<ComponentValue> = Vec::new();

  for child in block.value.into_iter() {
    match child {
      ComponentValue::AtRule(nested) => {
        if can_atomicify_at_rule(&nested) {
          let converted = atomicify_at_rule(*nested, options, ctx, &label);
          new_children.push(ComponentValue::AtRule(Box::new(converted)));
        } else {
          new_children.push(ComponentValue::AtRule(nested));
        }
      }
      ComponentValue::QualifiedRule(rule) => {
        let replacements = atomicify_qualified_rule(*rule, options, ctx, Some(&label));
        for replacement in replacements {
          new_children.push(ComponentValue::QualifiedRule(Box::new(replacement)));
        }
      }
      ComponentValue::Declaration(declaration) => {
        let declaration = *declaration;
        let atomic_rule = atomicify_declaration(&declaration, &[], options, ctx, Some(&label));
        new_children.push(ComponentValue::QualifiedRule(Box::new(atomic_rule)));
      }
      _ => {}
    }
  }

  block.value = new_children;
  at_rule.block = Some(block);
  at_rule
}

fn atomicify_declaration(
  declaration: &Declaration,
  selectors: &[String],
  options: &AtomicifyOptions<'_>,
  ctx: &mut TransformContext<'_>,
  at_rule_label: Option<&str>,
) -> QualifiedRule {
  let selector_text = build_atomic_selector(declaration, selectors, options, ctx, at_rule_label);
  let mut rule = parse_selector_as_rule(&selector_text);
  rule.block.value = vec![ComponentValue::Declaration(Box::new(declaration.clone()))];
  rule
}

fn build_atomic_selector(
  declaration: &Declaration,
  selectors: &[String],
  options: &AtomicifyOptions<'_>,
  ctx: &mut TransformContext<'_>,
  at_rule_label: Option<&str>,
) -> String {
  fn collapse_universal(selector: &str) -> String {
    let mut collapsed = selector.to_string();
    while collapsed.contains(" * *") {
      collapsed = collapsed.replace(" * *", " *");
    }
    collapsed
  }

  let base_selectors: Vec<Cow<'_, str>> = if selectors.is_empty() {
    vec![Cow::Borrowed("")]
  } else {
    selectors
      .iter()
      .map(|selector| Cow::Owned(collapse_universal(selector)))
      .collect()
  };
  if trace_enabled() {
    eprintln!(
      "[atomicify.selector] raw={:?} collapsed={:?}",
      selectors,
      base_selectors
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
    );
  }

  let mut built: Vec<String> = Vec::with_capacity(base_selectors.len());

  for selector in base_selectors {
    let normalized = normalize_selector(selector.as_ref());
    let class_name = atomic_class_name(declaration, options, &normalized, at_rule_label);
    ctx.push_class_name(class_name.clone());

    let replacement = options
      .class_name_compression_map
      .and_then(|map| map.get(&class_name[1..]))
      .cloned()
      .unwrap_or(class_name.clone());

    let replaced = replace_nesting_selector(&normalized, &replacement);
    if trace_enabled() {
      let prop = declaration_name(&declaration.name);
      eprintln!(
        "[atomicify] prop='{}' selector='{}' class='{}' replaced='{}'",
        prop, normalized, class_name, replaced
      );
    }
    built.push(replaced);
  }

  built.join(", ")
}

fn atomic_class_name(
  declaration: &Declaration,
  options: &AtomicifyOptions<'_>,
  normalized_selector: &str,
  at_rule_label: Option<&str>,
) -> String {
  let prefix = options.class_hash_prefix.unwrap_or("");
  let prop = declaration_name(&declaration.name);
  let at_rule = at_rule_label.unwrap_or("undefined");
  let group_seed = format!("{}{}{}{}", prefix, at_rule, normalized_selector, prop);
  let group_hash = hash(&group_seed);
  let group = group_hash.chars().take(4).collect::<String>();

  let mut value_seed = serialize_component_values(&declaration.value).unwrap_or_default();
  // COMPAT: Babel trims whitespace around multiplication inside calc() before hashing.
  value_seed = value_seed.replace(" *", "*");
  value_seed = value_seed.replace("* ", "*");
  value_seed = value_seed.replace("*-", "* -");
  value_seed = value_seed.replace("*+", "* +");
  if declaration.important.is_some() {
    value_seed.push_str("true");
  }
  if std::env::var("COMPILED_CLI_TRACE").is_ok()
    && declaration_name(&declaration.name) == "margin-left"
  {
    eprintln!("[atomicify.hash] raw='{}'", value_seed);
  }
  let value_hash = hash(&value_seed);
  let value = value_hash.chars().take(4).collect::<String>();

  format!("_{}{}", group, value)
}

fn replace_nesting_selector(selector: &str, parent_class_name: &str) -> String {
  let replacement = format!(".{}", parent_class_name);
  selector.replace('&', &replacement)
}

fn collapse_adjacent_ampersands(selector: &str) -> String {
  let mut out = String::with_capacity(selector.len());
  let mut chars = selector.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '&' {
      out.push('&');

      loop {
        let mut consumed_ws = false;
        while let Some(&next) = chars.peek() {
          if next.is_whitespace() {
            consumed_ws = true;
            chars.next();
          } else {
            break;
          }
        }

        match chars.peek() {
          Some('&') => {
            // Collapse any chain of ampersands separated by whitespace.
            chars.next();
            out.push('&');
            continue;
          }
          Some(_) => {
            if consumed_ws {
              // Preserve a single space when whitespace wasn't between two ampersands.
              out.push(' ');
            }
          }
          None => {
            // Do not emit trailing whitespace at end of selector.
          }
        }
        break;
      }

      continue;
    }

    out.push(ch);
  }

  out
}

fn normalize_selector(selector: &str) -> String {
  if selector.is_empty() {
    return "&".to_string();
  }

  let trimmed = selector.trim();
  if std::env::var("COMPILED_CSS_TRACE").is_ok() {
    eprintln!("[atomicify] normalize_selector input='{}'", trimmed);
  }
  if trimmed.contains('&') {
    return collapse_adjacent_ampersands(trimmed);
  }

  format!("& {}", trimmed)
}

fn declaration_name(name: &DeclarationName) -> String {
  match name {
    DeclarationName::Ident(ident) => ident.value.to_string(),
    DeclarationName::DashedIdent(ident) => ident.value.to_string(),
  }
}

#[allow(unreachable_patterns)]
fn collect_rule_selectors(rule: &QualifiedRule) -> Vec<String> {
  fn canonicalize(selector: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in selector.split_whitespace() {
      if part == "*" && out.last().copied() == Some("*") {
        continue;
      }
      out.push(part);
    }
    out.join(" ")
  }

  let selectors = match &rule.prelude {
    QualifiedRulePrelude::SelectorList(list) => list
      .children
      .iter()
      .map(serialize_complex_selector_with_possible_nesting)
      .collect::<Vec<String>>(),
    QualifiedRulePrelude::RelativeSelectorList(list) => list
      .children
      .iter()
      .map(|rel| serialize_complex_selector_with_possible_nesting(&rel.selector))
      .collect::<Vec<String>>(),
    QualifiedRulePrelude::ListOfComponentValues(list) => {
      if let Some(parsed) =
        crate::postcss::utils::selector_stringifier::parse_selector_list_from_component_values(list)
      {
        parsed
          .children
          .iter()
          .map(serialize_complex_selector_with_possible_nesting)
          .collect::<Vec<String>>()
      } else {
        serialize_component_values(&list.children)
          .into_iter()
          .collect()
      }
    }
    _ => Vec::new(),
  };
  if trace_enabled() {
    eprintln!("[atomicify.collect] raw selectors={:?}", selectors);
  }

  let mut dedup: indexmap::IndexSet<String> = indexmap::IndexSet::new();
  for selector in selectors {
    let mut canonical = canonicalize(&selector);
    // Collapse repeated universal descendants (e.g., "* *" -> "*") to mirror
    // postcss-nested JS behavior.
    while canonical.contains(" * *") {
      canonical = canonical.replace(" * *", " *");
    }
    dedup.insert(canonical);
  }

  dedup.into_iter().collect()
}

fn filter_redundant_selectors(selectors: Vec<String>) -> Vec<String> {
  use indexmap::IndexSet;
  eprintln!("[atomicify.filter] raw selectors={:?}", selectors);
  let mut set: IndexSet<String> = IndexSet::new();
  for sel in selectors {
    let collapsed_once = sel.replace("* *", "*");
    if collapsed_once != sel && set.contains(&collapsed_once) {
      continue;
    }
    set.insert(collapsed_once);
  }
  let result: Vec<String> = set.into_iter().collect();
  eprintln!("[atomicify.filter] filtered selectors={:?}", result);
  result
}

fn serialize_complex_selector_with_possible_nesting(
  selector: &swc_core::css::ast::ComplexSelector,
) -> String {
  crate::postcss::utils::selector_stringifier::serialize_complex_selector(selector)
}

fn serialize_node<T>(node: &T) -> Option<String>
where
  T: Spanned,
  for<'writer> CodeGenerator<BasicCssWriter<'writer, &'writer mut String>>: Emit<T>,
{
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
    if generator.emit(node).is_err() {
      return None;
    }
  }

  Some(output)
}

fn serialize_component_values(values: &[ComponentValue]) -> Option<String> {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });

    for component in values {
      if generator.emit(component).is_err() {
        return None;
      }
    }
  }

  Some(output)
}

fn parse_selector_as_rule(selector: &str) -> QualifiedRule {
  let css = format!("{}{{}}", selector);
  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom("atomic.css".into()).into(), css.into());
  let mut errors = vec![];
  let stylesheet = parse_string_input::<Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  )
  .expect("failed to parse atomic selector");

  if let Some(error) = errors.into_iter().next() {
    panic!("failed to parse selector: {error:?}");
  }

  match stylesheet.rules.into_iter().next() {
    Some(Rule::QualifiedRule(rule)) => *rule,
    _ => panic!("expected qualified rule"),
  }
}

fn can_atomicify_at_rule(at_rule: &AtRule) -> bool {
  let name = at_rule_name(&at_rule.name);
  let allowed = [
    "container",
    "-moz-document",
    "else",
    "layer",
    "media",
    "starting-style",
    "supports",
    "when",
  ];
  let forbidden = ["charset", "import", "namespace"];
  let ignored = [
    "color-profile",
    "counter-style",
    "font-face",
    "font-palette-values",
    "keyframes",
    "page",
    "property",
  ];

  if allowed.contains(&name.as_str()) {
    true
  } else if forbidden.contains(&name.as_str()) {
    panic!("At-rule '@{}' cannot be used in CSS rules.", name);
  } else if !ignored.contains(&name.as_str()) {
    panic!("Unknown at-rule '@{}'.", name);
  } else {
    false
  }
}

fn at_rule_name(name: &AtRuleName) -> String {
  match name {
    AtRuleName::Ident(ident) => ident.value.to_string(),
    AtRuleName::DashedIdent(ident) => ident.value.to_string(),
  }
}

fn at_rule_params(at_rule: &AtRule) -> String {
  at_rule
    .prelude
    .as_ref()
    .map(|prelude| serialize_at_rule_prelude(prelude).trim().to_string())
    .unwrap_or_default()
}

fn serialize_at_rule_prelude(prelude: &AtRulePrelude) -> String {
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

fn is_comment_list(_list: &ListOfComponentValues) -> bool {
  false
}

fn is_css_identifier_valid(value: &str) -> bool {
  let mut chars = value.chars();
  match chars.next() {
    Some(first) if is_identifier_start(first) => chars.all(is_identifier_continue),
    _ => false,
  }
}

fn is_identifier_start(ch: char) -> bool {
  ch.is_ascii_alphabetic() || ch == '-' || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
  ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};

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
  fn hash_matches_js_port() {
    assert_eq!(hash("color"), "1ylxx6h");
    assert_eq!(hash("margin"), "1py5azy");
    assert_eq!(hash("!important"), "pjhvf0");
  }

  #[test]
  fn builds_atomic_rules_for_basic_declaration() {
    let mut stylesheet = parse_stylesheet(".foo { color: red; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    AtomicifyRules.run(&mut stylesheet, &mut ctx);

    assert_eq!(stylesheet.rules.len(), 1);
    if let Rule::QualifiedRule(rule) = &stylesheet.rules[0] {
      let selectors = collect_rule_selectors(rule);
      assert_eq!(selectors.len(), 1);
      assert!(selectors[0].contains("._"));
    } else {
      panic!("expected qualified rule");
    }
  }

  #[test]
  fn parse_selector_as_rule_produces_selector_list() {
    let rule = parse_selector_as_rule("._foo>*");
    match rule.prelude {
      QualifiedRulePrelude::SelectorList(_) => {}
      other => panic!("expected selector list, got {:?}", other),
    }
  }
}
fn trace_enabled() -> bool {
  std::env::var("COMPILED_CSS_TRACE").is_ok()
}
