use swc_core::atoms::Atom;
use swc_core::common::DUMMY_SP;
use swc_core::css::ast::{
  ComponentValue, Declaration, Ident, KeyframeBlock, QualifiedRule, Rule, SimpleBlock, Stylesheet,
};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct NormalizeCurrentColor;

impl Plugin for NormalizeCurrentColor {
  fn name(&self) -> &'static str {
    "normalize-current-color"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    normalize_stylesheet(stylesheet);
  }
}

pub fn normalize_current_color() -> NormalizeCurrentColor {
  NormalizeCurrentColor
}

fn normalize_stylesheet(stylesheet: &mut Stylesheet) {
  for rule in &mut stylesheet.rules {
    match rule {
      Rule::QualifiedRule(rule) => normalize_qualified_rule(rule),
      Rule::AtRule(rule) => {
        if let Some(block) = &mut rule.block {
          normalize_simple_block(block);
        }
      }
      Rule::ListOfComponentValues(list) => normalize_component_values(&mut list.children),
    }
  }
}

fn normalize_qualified_rule(rule: &mut QualifiedRule) {
  normalize_simple_block(&mut rule.block);
}

fn normalize_simple_block(block: &mut SimpleBlock) {
  normalize_component_values(&mut block.value);
}

fn normalize_keyframe_block(block: &mut KeyframeBlock) {
  normalize_simple_block(&mut block.block);
}

fn normalize_component_values(values: &mut Vec<ComponentValue>) {
  for component in values.iter_mut() {
    match component {
      ComponentValue::Declaration(declaration) => normalize_declaration(declaration),
      ComponentValue::SimpleBlock(block) => normalize_simple_block(block),
      ComponentValue::QualifiedRule(rule) => normalize_qualified_rule(rule),
      ComponentValue::KeyframeBlock(block) => normalize_keyframe_block(block),
      ComponentValue::Function(function) => normalize_component_values(&mut function.value),
      ComponentValue::ListOfComponentValues(list) => normalize_component_values(&mut list.children),
      ComponentValue::AtRule(rule) => {
        if let Some(block) = &mut rule.block {
          normalize_simple_block(block);
        }
      }
      _ => {}
    }
  }
}

fn normalize_declaration(declaration: &mut Declaration) {
  if declaration.value.is_empty() {
    return;
  }

  let Some(serialized) = serialize_component_values(&declaration.value) else {
    return;
  };

  let trimmed = serialized.trim();
  if trimmed.is_empty() {
    return;
  }

  let lower = trimmed.to_ascii_lowercase();
  if lower == "currentcolor" || lower == "current-color" {
    declaration.value = vec![ComponentValue::Ident(Box::new(Ident {
      span: DUMMY_SP,
      value: Atom::from("currentColor"),
      raw: None,
    }))];
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

  fn collect_first_declaration_value(stylesheet: &Stylesheet) -> Option<String> {
    for rule in &stylesheet.rules {
      if let Rule::QualifiedRule(rule) = rule {
        for component in &rule.block.value {
          if let ComponentValue::Declaration(declaration) = component {
            return serialize_component_values(&declaration.value);
          }
        }
      }
    }
    None
  }

  #[test]
  fn normalizes_lowercase_currentcolor() {
    let mut stylesheet = parse_stylesheet(".a { color: currentcolor; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    NormalizeCurrentColor.run(&mut stylesheet, &mut ctx);

    let value = collect_first_declaration_value(&stylesheet).expect("value");
    assert_eq!(value, "currentColor");
  }

  #[test]
  fn normalizes_hyphenated_current_color() {
    let mut stylesheet = parse_stylesheet(".a { color: current-color; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    NormalizeCurrentColor.run(&mut stylesheet, &mut ctx);

    let value = collect_first_declaration_value(&stylesheet).expect("value");
    assert_eq!(value, "currentColor");
  }
}
