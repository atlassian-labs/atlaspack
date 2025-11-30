use swc_core::common::Spanned;
use swc_core::css::ast::{ComponentValue, QualifiedRule, QualifiedRulePrelude, Rule};
use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};

use super::style_ordering::STYLE_ORDER;

/// Sort the provided rules in-place based on the LVFHA ordering used by the
/// existing PostCSS implementation.
pub fn sort_pseudo_selectors(rules: &mut Vec<Rule>) {
  rules.sort_by(|a, b| pseudo_selector_score(a).cmp(&pseudo_selector_score(b)));
}

fn pseudo_selector_score(rule: &Rule) -> usize {
  first_selector(rule)
    .map(|selector| selector_score(selector.trim()))
    .unwrap_or(0)
}

fn selector_score(selector: &str) -> usize {
  STYLE_ORDER
    .iter()
    .position(|pseudo| selector.ends_with(pseudo))
    .map(|idx| idx + 1)
    .unwrap_or(0)
}

fn first_selector(rule: &Rule) -> Option<String> {
  match rule {
    Rule::QualifiedRule(rule) => collect_rule_selectors(rule).into_iter().next(),
    _ => None,
  }
}

pub(crate) fn collect_rule_selectors(rule: &QualifiedRule) -> Vec<String> {
  match &rule.prelude {
    QualifiedRulePrelude::SelectorList(list) => list
      .children
      .iter()
      .filter_map(|selector| serialize_node(selector))
      .collect(),
    QualifiedRulePrelude::RelativeSelectorList(list) => list
      .children
      .iter()
      .filter_map(|selector| serialize_node(selector))
      .collect(),
    QualifiedRulePrelude::ListOfComponentValues(list) => serialize_component_values(&list.children)
      .into_iter()
      .collect(),
  }
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
