use std::sync::Arc;

use swc_core::common::{FileName, SourceMap, input::StringInput};
use swc_core::css::ast::{QualifiedRule, QualifiedRulePrelude, Rule, Stylesheet};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

use crate::postcss::plugins::extract_stylesheets::normalize_block_value_spacing;

fn serialize_rule(rule: &QualifiedRule) -> Option<String> {
  let mut output = String::new();
  let writer = BasicCssWriter::new(&mut output, None, Default::default());
  let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
  generator.emit(rule).ok()?;
  Some(output)
}

/// Splits a comma-separated atomic sheet into individual selectors, preserving
/// declarations. Returns the original sheet when parsing fails or only a single
/// selector is present.
pub fn split_atomic_sheet(sheet: &str) -> Vec<String> {
  if !sheet.contains(',') || sheet.trim_start().starts_with('@') {
    return vec![sheet.to_string()];
  }

  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(
    FileName::Custom("atomic.css".into()).into(),
    sheet.to_string(),
  );
  let mut errors = vec![];
  let stylesheet = match parse_string_input::<Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  ) {
    Ok(sheet) if errors.is_empty() => sheet,
    _ => return vec![sheet.to_string()],
  };

  match stylesheet.rules.into_iter().next() {
    Some(Rule::QualifiedRule(rule)) => match &rule.prelude {
      QualifiedRulePrelude::SelectorList(list) => {
        if list.children.len() <= 1 {
          vec![sheet.to_string()]
        } else {
          let mut result = Vec::with_capacity(list.children.len());
          for selector in &list.children {
            let mut new_rule = rule.clone();
            let mut new_list = list.clone();
            new_list.children = vec![selector.clone()];
            new_rule.prelude = QualifiedRulePrelude::SelectorList(new_list);
            if let Some(serialized) = serialize_rule(&new_rule) {
              result.push(normalize_block_value_spacing(&serialized));
            } else {
              return vec![sheet.to_string()];
            }
          }
          result
        }
      }
      _ => vec![sheet.to_string()],
    },
    _ => vec![sheet.to_string()],
  }
}
