use swc_core::atoms::Atom;
use swc_core::css::ast::{
  ComponentValue, Ident, KeyframeBlock, KeyframeSelector, QualifiedRule, QualifiedRulePrelude,
  RelativeSelectorList, Rule, SelectorList, SimpleBlock, Stylesheet,
};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct FlattenMultipleSelectors;

impl Plugin for FlattenMultipleSelectors {
  fn name(&self) -> &'static str {
    "flatten-multiple-selectors"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    flatten_stylesheet(stylesheet);
  }
}

pub fn flatten_multiple_selectors() -> FlattenMultipleSelectors {
  FlattenMultipleSelectors
}

fn flatten_stylesheet(stylesheet: &mut Stylesheet) {
  flatten_rules(&mut stylesheet.rules);
}

fn flatten_rules(rules: &mut Vec<Rule>) {
  let mut index = 0;

  while index < rules.len() {
    let replacements = match &rules[index] {
      Rule::QualifiedRule(rule) => split_qualified_rule(rule),
      _ => None,
    };

    if let Some(mut replacement_rules) = replacements {
      // Remove the original rule and replace it with cloned rules that only contain a single selector.
      rules.remove(index);
      let count = replacement_rules.len();
      for (offset, replacement) in replacement_rules.drain(..).enumerate() {
        rules.insert(index + offset, Rule::QualifiedRule(Box::new(replacement)));
      }
      index += count;
      continue;
    }

    match &mut rules[index] {
      Rule::QualifiedRule(rule) => flatten_simple_block(&mut rule.block),
      Rule::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          flatten_simple_block(block);
        }
      }
      Rule::ListOfComponentValues(list) => flatten_component_values(&mut list.children),
    }

    index += 1;
  }
}

fn flatten_simple_block(block: &mut SimpleBlock) {
  flatten_component_values(&mut block.value);
}

fn flatten_component_values(values: &mut Vec<ComponentValue>) {
  let mut index = 0;

  while index < values.len() {
    if let ComponentValue::QualifiedRule(rule) = &values[index] {
      if let Some(mut replacement_rules) = split_qualified_rule(rule) {
        values.remove(index);
        let count = replacement_rules.len();
        for (offset, replacement) in replacement_rules.drain(..).enumerate() {
          values.insert(
            index + offset,
            ComponentValue::QualifiedRule(Box::new(replacement)),
          );
        }
        index += count;
        continue;
      }
    }

    if let ComponentValue::KeyframeBlock(block) = &values[index] {
      if let Some(mut replacement_blocks) = split_keyframe_block(block) {
        values.remove(index);
        let count = replacement_blocks.len();
        for (offset, replacement) in replacement_blocks.drain(..).enumerate() {
          values.insert(
            index + offset,
            ComponentValue::KeyframeBlock(Box::new(replacement)),
          );
        }
        index += count;
        continue;
      }
    }

    match &mut values[index] {
      ComponentValue::QualifiedRule(rule) => flatten_simple_block(&mut rule.block),
      ComponentValue::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          flatten_simple_block(block);
        }
      }
      ComponentValue::SimpleBlock(block) => flatten_component_values(&mut block.value),
      ComponentValue::ListOfComponentValues(list) => flatten_component_values(&mut list.children),
      ComponentValue::Function(function) => flatten_component_values(&mut function.value),
      ComponentValue::KeyframeBlock(block) => flatten_simple_block(&mut block.block),
      _ => {}
    }

    index += 1;
  }
}

fn split_qualified_rule(rule: &QualifiedRule) -> Option<Vec<QualifiedRule>> {
  match &rule.prelude {
    QualifiedRulePrelude::SelectorList(list) => split_selector_list(rule, list),
    QualifiedRulePrelude::RelativeSelectorList(list) => split_relative_selector_list(rule, list),
    _ => None,
  }
}

fn split_keyframe_block(block: &KeyframeBlock) -> Option<Vec<KeyframeBlock>> {
  if block.prelude.len() <= 1 {
    return None;
  }

  let mut replacements = Vec::with_capacity(block.prelude.len());

  for selector in &block.prelude {
    let mut new_block = block.clone();
    new_block.prelude = vec![selector.clone()];
    normalize_keyframe_selectors(&mut new_block.prelude);
    replacements.push(new_block);
  }

  Some(replacements)
}

fn normalize_keyframe_selectors(selectors: &mut [KeyframeSelector]) {
  for selector in selectors {
    if let KeyframeSelector::Percentage(percentage) = selector {
      if (percentage.value.value - 100.0).abs() < f64::EPSILON {
        *selector = KeyframeSelector::Ident(Ident {
          span: percentage.span,
          value: Atom::from("to"),
          raw: None,
        });
      }
    }
  }
}

fn split_selector_list(rule: &QualifiedRule, list: &SelectorList) -> Option<Vec<QualifiedRule>> {
  if list.children.len() <= 1 {
    return None;
  }

  let mut replacements = Vec::with_capacity(list.children.len());

  for selector in &list.children {
    let mut new_rule = rule.clone();
    let mut new_list = list.clone();
    new_list.children = vec![selector.clone()];
    new_rule.prelude = QualifiedRulePrelude::SelectorList(new_list);
    replacements.push(new_rule);
  }

  Some(replacements)
}

fn split_relative_selector_list(
  rule: &QualifiedRule,
  list: &RelativeSelectorList,
) -> Option<Vec<QualifiedRule>> {
  if list.children.len() <= 1 {
    return None;
  }

  let mut replacements = Vec::with_capacity(list.children.len());

  for relative in &list.children {
    let mut new_rule = rule.clone();
    let mut new_list = list.clone();
    new_list.children = vec![relative.clone()];
    new_rule.prelude = QualifiedRulePrelude::RelativeSelectorList(new_list);
    replacements.push(new_rule);
  }

  Some(replacements)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use swc_core::common::{input::StringInput, FileName, SourceMap};
  use swc_core::css::ast::{ComplexSelector, KeyframeSelector, QualifiedRulePrelude, Rule};
  use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};
  use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

  fn parse_stylesheet(css: &str) -> Stylesheet {
    let cm: std::sync::Arc<SourceMap> = Default::default();
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

  fn serialize_selector(selector: &ComplexSelector) -> String {
    let mut output = String::new();
    {
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
      generator
        .emit(selector)
        .expect("failed to serialize complex selector");
    }
    output
  }

  fn serialize_keyframe_selector(selector: &KeyframeSelector) -> String {
    let mut output = String::new();
    {
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
      generator
        .emit(selector)
        .expect("failed to serialize keyframe selector");
    }
    output
  }

  fn collect_rule_selectors(rule: &QualifiedRule) -> Vec<String> {
    match &rule.prelude {
      QualifiedRulePrelude::SelectorList(list) => {
        list.children.iter().map(serialize_selector).collect()
      }
      QualifiedRulePrelude::RelativeSelectorList(list) => list
        .children
        .iter()
        .map(|relative| serialize_selector(&relative.selector))
        .collect(),
      _ => vec![],
    }
  }

  #[test]
  fn splits_top_level_multiple_selectors_into_individual_rules() {
    let mut stylesheet = parse_stylesheet(".a, .b { color: red; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    FlattenMultipleSelectors.run(&mut stylesheet, &mut ctx);

    let selectors: Vec<Vec<String>> = stylesheet
      .rules
      .iter()
      .filter_map(|rule| match rule {
        Rule::QualifiedRule(rule) => Some(collect_rule_selectors(rule)),
        _ => None,
      })
      .collect();

    assert_eq!(
      selectors,
      vec![vec![String::from(".a")], vec![String::from(".b")]]
    );
  }

  #[test]
  fn splits_nested_multiple_selectors_inside_at_rules() {
    let mut stylesheet = parse_stylesheet("@media screen { .a, .b { color: blue; } }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    FlattenMultipleSelectors.run(&mut stylesheet, &mut ctx);

    let mut nested_selectors = Vec::new();
    for rule in &stylesheet.rules {
      if let Rule::AtRule(at_rule) = rule {
        if let Some(block) = &at_rule.block {
          for component in &block.value {
            if let ComponentValue::QualifiedRule(rule) = component {
              nested_selectors.push(collect_rule_selectors(rule));
            }
          }
        }
      }
    }

    assert_eq!(
      nested_selectors,
      vec![vec![String::from(".a")], vec![String::from(".b")]]
    );
  }

  #[test]
  fn splits_keyframe_block_selectors() {
    let mut stylesheet = parse_stylesheet("@keyframes pulse { 0%, 50% { opacity: 0; } }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    FlattenMultipleSelectors.run(&mut stylesheet, &mut ctx);

    let mut selector_lists = Vec::new();
    for rule in &stylesheet.rules {
      if let Rule::AtRule(at_rule) = rule {
        if let Some(block) = &at_rule.block {
          for component in &block.value {
            if let ComponentValue::KeyframeBlock(block) = component {
              selector_lists.push(
                block
                  .prelude
                  .iter()
                  .map(serialize_keyframe_selector)
                  .collect::<Vec<_>>(),
              );
            }
          }
        }
      }
    }

    assert_eq!(
      selector_lists,
      vec![vec![String::from("0%")], vec![String::from("50%")]]
    );
  }

  #[test]
  fn handles_relative_selectors() {
    let mut stylesheet = parse_stylesheet(".a { &:hover, &:focus { color: green; } }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    FlattenMultipleSelectors.run(&mut stylesheet, &mut ctx);

    let mut nested_selectors = Vec::new();
    for rule in &stylesheet.rules {
      if let Rule::QualifiedRule(rule) = rule {
        for component in &rule.block.value {
          if let ComponentValue::QualifiedRule(nested) = component {
            nested_selectors.push(collect_rule_selectors(nested));
          }
        }
      }
    }

    assert_eq!(
      nested_selectors,
      vec![vec![String::from("&:hover")], vec![String::from("&:focus")]]
    );
  }
}
