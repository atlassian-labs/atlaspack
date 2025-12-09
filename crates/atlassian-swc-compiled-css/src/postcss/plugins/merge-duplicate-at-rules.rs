use indexmap::IndexMap;
use swc_core::css::ast::{AtRule, AtRuleName, AtRulePrelude, ComponentValue, Rule, Stylesheet};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};

use super::super::transform::{Plugin, TransformContext};
use swc_core::css::ast::SimpleBlock;

#[derive(Debug, Default, Clone, Copy)]
pub struct MergeDuplicateAtRules;

impl Plugin for MergeDuplicateAtRules {
  fn name(&self) -> &'static str {
    "merge-duplicate-at-rules"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    let mut store: IndexMap<String, MergeEntry> = IndexMap::new();
    let mut retained_rules: Vec<Rule> = Vec::with_capacity(stylesheet.rules.len());

    for rule in std::mem::take(&mut stylesheet.rules) {
      match rule {
        Rule::AtRule(at_rule) => {
          merge_at_rule(*at_rule, &mut store);
        }
        Rule::QualifiedRule(mut qualified) => {
          extract_from_block(&mut qualified.block, &mut store);
          retained_rules.push(Rule::QualifiedRule(qualified));
        }
        Rule::ListOfComponentValues(mut list) => {
          extract_from_component_values(&mut list.children, &mut store);
          retained_rules.push(Rule::ListOfComponentValues(list));
        }
      }
    }

    for (_key, mut entry) in store.into_iter() {
      if let Some(block) = entry.at_rule.block.as_mut() {
        block.value = entry.children.into_iter().map(|(_, child)| child).collect();
      }
      retained_rules.push(Rule::AtRule(Box::new(entry.at_rule)));
    }

    stylesheet.rules = retained_rules;
  }
}

pub fn merge_duplicate_at_rules() -> MergeDuplicateAtRules {
  MergeDuplicateAtRules
}

struct MergeEntry {
  at_rule: AtRule,
  children: IndexMap<String, ComponentValue>,
}

fn merge_at_rule(mut at_rule: AtRule, store: &mut IndexMap<String, MergeEntry>) {
  let key = format!(
    "{}{}",
    at_rule_name(&at_rule.name),
    at_rule_params(&at_rule)
  );

  let mut collected_children: Vec<(String, ComponentValue)> = Vec::new();

  if let Some(mut block) = at_rule.block.take() {
    let span = block.span;
    let name = block.name;

    for child in block.value.drain(..) {
      let key = serialize_component_value(&child);
      collected_children.push((key, child));
    }

    at_rule.block = Some(SimpleBlock {
      span,
      name,
      value: Vec::new(),
    });
  }

  let entry = store.entry(key).or_insert_with(|| MergeEntry {
    at_rule,
    children: IndexMap::new(),
  });

  for (child_key, child) in collected_children {
    entry.children.entry(child_key).or_insert(child);
  }
}

fn extract_from_block(block: &mut SimpleBlock, store: &mut IndexMap<String, MergeEntry>) {
  extract_from_component_values(&mut block.value, store);
}

fn extract_from_component_values(
  values: &mut Vec<ComponentValue>,
  store: &mut IndexMap<String, MergeEntry>,
) {
  let mut index = 0;
  while index < values.len() {
    match values[index] {
      ComponentValue::AtRule(_) => {
        if let ComponentValue::AtRule(at_rule) = values.remove(index) {
          merge_at_rule(*at_rule, store);
        }
        continue;
      }
      ComponentValue::QualifiedRule(ref mut rule) => {
        extract_from_block(&mut rule.block, store);
      }
      ComponentValue::SimpleBlock(ref mut block) => {
        extract_from_component_values(&mut block.value, store);
      }
      ComponentValue::Function(ref mut function) => {
        extract_from_component_values(&mut function.value, store);
      }
      ComponentValue::ListOfComponentValues(ref mut list) => {
        extract_from_component_values(&mut list.children, store);
      }
      ComponentValue::KeyframeBlock(ref mut block) => {
        extract_from_block(&mut block.block, store);
      }
      _ => {}
    }

    index += 1;
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
    .map(|prelude| serialize_prelude(prelude).trim().to_string())
    .unwrap_or_default()
}

fn serialize_component_value(value: &ComponentValue) -> String {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
    generator
      .emit(value)
      .expect("failed to serialize CSS component value");
  }
  output
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

fn serialize_stylesheet(stylesheet: &Stylesheet) -> String {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
    generator
      .emit(stylesheet)
      .expect("failed to serialize stylesheet");
  }
  output
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use swc_core::common::{FileName, SourceMap, input::StringInput};
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

  fn transform(css: &str) -> Stylesheet {
    let mut stylesheet = parse_stylesheet(css);
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    MergeDuplicateAtRules.run(&mut stylesheet, &mut ctx);
    stylesheet
  }

  #[test]
  fn removes_duplicate_children() {
    let stylesheet = transform(
      "@media (min-width:500px){._171dak0l{border:2px solid red}}\n@media (min-width:500px){._171dak0l{border:2px solid red}._1swkri7e:before{content:'large screen'}}",
    );

    assert_media_rule_children(&stylesheet);
  }

  #[test]
  fn removes_duplicate_children_with_different_order() {
    let stylesheet = transform(
      "@media (min-width:500px){._171dak0l{border:2px solid red}._1swkri7e:before{content:'large screen'}}\n@media (min-width:500px){._171dak0l{border:2px solid red}}",
    );

    assert_media_rule_children(&stylesheet);
  }

  fn assert_media_rule_children(stylesheet: &Stylesheet) {
    assert_eq!(
      stylesheet.rules.len(),
      1,
      "expected a single top-level rule"
    );

    let at_rule = match &stylesheet.rules[0] {
      Rule::AtRule(rule) => rule,
      _ => panic!("expected top-level at-rule"),
    };

    let block = at_rule
      .block
      .as_ref()
      .expect("expected media query to have a block");

    let children: Vec<String> = block
      .value
      .iter()
      .map(super::serialize_component_value)
      .collect();

    assert_eq!(children.len(), 2, "expected two unique child rules");
    assert_eq!(
      children,
      vec![
        "._171dak0l{border:2px solid red}".to_string(),
        "._1swkri7e:before{content:\"large screen\"}".to_string(),
      ]
    );
  }
}
