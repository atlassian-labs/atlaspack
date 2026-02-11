use swc_core::css::ast::{
  ComponentValue, Declaration, DeclarationName, Ident, QualifiedRule, Rule, Stylesheet,
};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct VendorPrefixingLite;

pub fn vendor_prefixing_lite() -> VendorPrefixingLite {
  VendorPrefixingLite
}

impl Plugin for VendorPrefixingLite {
  fn name(&self) -> &'static str {
    "vendor-prefixing-lite"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    let mut out: Vec<Rule> = Vec::with_capacity(stylesheet.rules.len());
    for rule in std::mem::take(&mut stylesheet.rules) {
      match rule {
        Rule::QualifiedRule(mut qr) => {
          apply_prefixing_to_rule(&mut qr);
          out.push(Rule::QualifiedRule(qr));
        }
        Rule::AtRule(mut ar) => {
          if let Some(mut block) = ar.block.take() {
            let mut new_children: Vec<ComponentValue> = Vec::with_capacity(block.value.len());
            for child in block.value.into_iter() {
              match child {
                ComponentValue::QualifiedRule(mut qr) => {
                  apply_prefixing_to_rule(&mut qr);
                  new_children.push(ComponentValue::QualifiedRule(qr));
                }
                other => new_children.push(other),
              }
            }
            block.value = new_children;
            ar.block = Some(block);
          }
          out.push(Rule::AtRule(ar));
        }
        other => out.push(other),
      }
    }
    stylesheet.rules = out;
  }
}

fn decl_name(name: &DeclarationName) -> &str {
  match name {
    DeclarationName::Ident(i) => &i.value,
    DeclarationName::DashedIdent(i) => &i.value,
  }
}

fn is_min_max_width_like(name: &str) -> bool {
  matches!(name, "width" | "min-width" | "max-width")
}

fn value_is_ident(list: &Vec<ComponentValue>, expected: &str) -> bool {
  if list.len() != 1 {
    return false;
  }
  match &list[0] {
    ComponentValue::Ident(i) => i.value.eq_ignore_ascii_case(expected),
    _ => false,
  }
}

pub fn make_ident(value: &str) -> ComponentValue {
  ComponentValue::Ident(Box::new(Ident {
    value: value.into(),
    raw: None,
    span: Default::default(),
  }))
}

fn clone_with_value(decl: &Declaration, new_value: ComponentValue) -> Declaration {
  let mut cloned = decl.clone();
  cloned.value = vec![new_value];
  cloned
}

fn apply_prefixing_to_rule(rule: &mut QualifiedRule) {
  // Insert prefixed declarations before the original when needed.
  let mut new_block: Vec<ComponentValue> = Vec::with_capacity(rule.block.value.len());
  for node in std::mem::take(&mut rule.block.value) {
    if let ComponentValue::Declaration(decl_box) = &node {
      let decl = &**decl_box;
      let name = decl_name(&decl.name);
      if is_min_max_width_like(name) && value_is_ident(&decl.value, "fit-content") {
        // Mirror Babel/autoprefixer ordering: prefixed first, then unprefixed
        let moz_decl = clone_with_value(decl, make_ident("-moz-fit-content"));
        new_block.push(ComponentValue::Declaration(Box::new(moz_decl)));
      }
    }
    new_block.push(node);
  }
  rule.block.value = new_block;
}
