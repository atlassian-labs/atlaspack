use std::collections::HashSet;

use swc_core::css::ast::{
  ComponentValue, Declaration, DeclarationName, QualifiedRule, Rule, Stylesheet,
};
use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct DiscardDuplicates;

impl Plugin for DiscardDuplicates {
  fn name(&self) -> &'static str {
    "discard-duplicates"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    discard_top_level_duplicates(stylesheet, _ctx);
  }
}

pub fn discard_duplicates() -> DiscardDuplicates {
  DiscardDuplicates
}

fn discard_top_level_duplicates(stylesheet: &mut Stylesheet, ctx: &TransformContext<'_>) {
  // 1:1 with JS plugin semantics:
  // - Remove duplicate declarations that exist at the root level (when input is a flat list
  //   of declarations)
  // - Additionally, when the input was wrapped in a placeholder selector to allow parsing
  //   of declaration lists (SWC pipeline), perform the same duplicate removal within that
  //   specific rule block only.

  let placeholder = ctx.options.declaration_placeholder.as_deref();

  for rule in &mut stylesheet.rules {
    match rule {
      Rule::ListOfComponentValues(list) => {
        remove_duplicate_declarations(&mut list.children);
      }
      Rule::QualifiedRule(qrule) => {
        if let Some(ph) = placeholder {
          if qualified_rule_has_selector(qrule, ph) {
            remove_duplicate_declarations(&mut qrule.block.value);
          }
        }
      }
      _ => {}
    }
  }
}

fn remove_duplicate_declarations(values: &mut Vec<ComponentValue>) {
  if values.is_empty() {
    return;
  }

  let mut retain_mask = vec![false; values.len()];
  let mut seen = HashSet::new();

  for (index, component) in values.iter().enumerate().rev() {
    match component {
      ComponentValue::Declaration(declaration) => {
        if let Some(name) = declaration_name_key(&declaration) {
          if seen.insert(name) {
            retain_mask[index] = true;
          }
        } else {
          retain_mask[index] = true;
        }
      }
      _ => {
        retain_mask[index] = true;
      }
    }
  }

  let mut cursor = 0usize;
  values.retain_mut(|_| {
    let keep = retain_mask[cursor];
    cursor += 1;
    keep
  });
}

fn declaration_name_key(declaration: &Declaration) -> Option<String> {
  match &declaration.name {
    DeclarationName::Ident(ident) => Some(ident.value.to_string()),
    DeclarationName::DashedIdent(ident) => Some(ident.value.to_string()),
  }
}

fn qualified_rule_has_selector(rule: &QualifiedRule, expected_selector: &str) -> bool {
  // Serialize the prelude (selector list) and compare textually to the expected placeholder
  // (e.g. ".__compiled_declaration_wrapper__").
  // Serialize the entire prelude for reliable comparison across variants.
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
    let _ = generator.emit(&rule.prelude);
  }
  output.trim() == expected_selector
}
