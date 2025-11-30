use swc_core::css::ast::{
  ComponentValue, KeyframeBlock, QualifiedRule, Rule, SimpleBlock, Stylesheet, Token,
};
use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct DiscardEmptyRules;

impl Plugin for DiscardEmptyRules {
  fn name(&self) -> &'static str {
    "discard-empty-rules"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    prune_stylesheet(stylesheet);
  }
}

pub fn discard_empty_rules() -> DiscardEmptyRules {
  DiscardEmptyRules
}

fn prune_stylesheet(stylesheet: &mut Stylesheet) {
  stylesheet.rules.retain_mut(|rule| match rule {
    Rule::QualifiedRule(rule) => !prune_qualified_rule(rule),
    Rule::AtRule(rule) => {
      if let Some(block) = &mut rule.block {
        prune_simple_block(block);
      }

      true
    }
    Rule::ListOfComponentValues(list) => {
      prune_component_values(&mut list.children);
      !list.children.is_empty()
    }
  });
}

fn prune_qualified_rule(rule: &mut QualifiedRule) -> bool {
  prune_simple_block(&mut rule.block)
}

fn prune_simple_block(block: &mut SimpleBlock) -> bool {
  prune_component_values(&mut block.value)
}

fn prune_keyframe_block(block: &mut KeyframeBlock) -> bool {
  prune_simple_block(&mut block.block)
}

fn prune_component_values(values: &mut Vec<ComponentValue>) -> bool {
  values.retain_mut(|component| match component {
    ComponentValue::Declaration(declaration) => !declaration_value_is_empty(&declaration.value),
    ComponentValue::SimpleBlock(block) => !prune_simple_block(block),
    ComponentValue::QualifiedRule(rule) => !prune_qualified_rule(rule),
    ComponentValue::KeyframeBlock(block) => !prune_keyframe_block(block),
    ComponentValue::ListOfComponentValues(list) => {
      prune_component_values(&mut list.children);
      !list.children.is_empty()
    }
    ComponentValue::Function(function) => {
      prune_component_values(&mut function.value);
      true
    }
    ComponentValue::AtRule(rule) => {
      if let Some(block) = &mut rule.block {
        prune_simple_block(block);
      }

      true
    }
    _ => true,
  });

  values.is_empty()
}

fn declaration_value_is_empty(values: &[ComponentValue]) -> bool {
  if values.is_empty() {
    return true;
  }

  if values
        .iter()
        .all(|component| matches!(component, ComponentValue::PreservedToken(token) if is_whitespace_token(&token.token)))
    {
        return true;
    }

  match serialize_component_values(values) {
    Some(serialized) => {
      let trimmed = serialized.trim();
      trimmed.is_empty() || trimmed == "undefined" || trimmed == "null"
    }
    None => false,
  }
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

fn is_whitespace_token(token: &Token) -> bool {
  matches!(token, Token::WhiteSpace { .. })
}
