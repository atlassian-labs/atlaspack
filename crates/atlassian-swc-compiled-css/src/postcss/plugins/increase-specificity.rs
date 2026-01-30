use swc_core::atoms::Atom;
use swc_core::common::DUMMY_SP;
use swc_core::css::ast::{
  ComplexSelector, ComplexSelectorChildren, ComponentValue, CompoundSelector, IdSelector,
  PseudoClassSelector, PseudoClassSelectorChildren, PseudoElementSelector,
  PseudoElementSelectorChildren, QualifiedRule, QualifiedRulePrelude, Rule, SelectorList,
  SimpleBlock, Stylesheet, SubclassSelector,
};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct IncreaseSpecificity;

impl Plugin for IncreaseSpecificity {
  fn name(&self) -> &'static str {
    "increase-specificity"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    increase_stylesheet_specificity(stylesheet);
  }
}

pub fn increase_specificity() -> IncreaseSpecificity {
  IncreaseSpecificity
}

fn increase_stylesheet_specificity(stylesheet: &mut Stylesheet) {
  for rule in &mut stylesheet.rules {
    increase_rule_specificity(rule);
  }
}

fn increase_rule_specificity(rule: &mut Rule) {
  match rule {
    Rule::QualifiedRule(rule) => {
      increase_qualified_rule_specificity(rule);
      increase_simple_block_specificity(&mut rule.block);
    }
    Rule::AtRule(at_rule) => {
      if let Some(block) = &mut at_rule.block {
        increase_simple_block_specificity(block);
      }
    }
    Rule::ListOfComponentValues(list) => increase_component_values_specificity(&mut list.children),
  }
}

fn increase_simple_block_specificity(block: &mut SimpleBlock) {
  increase_component_values_specificity(&mut block.value);
}

fn increase_component_values_specificity(values: &mut [ComponentValue]) {
  for component in values {
    match component {
      ComponentValue::QualifiedRule(rule) => {
        increase_qualified_rule_specificity(rule);
        increase_simple_block_specificity(&mut rule.block);
      }
      ComponentValue::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          increase_simple_block_specificity(block);
        }
      }
      ComponentValue::SimpleBlock(block) => increase_simple_block_specificity(block),
      ComponentValue::Function(function) => {
        increase_component_values_specificity(&mut function.value)
      }
      ComponentValue::ListOfComponentValues(list) => {
        increase_component_values_specificity(&mut list.children)
      }
      ComponentValue::KeyframeBlock(block) => increase_simple_block_specificity(&mut block.block),
      _ => {}
    }
  }
}

fn increase_qualified_rule_specificity(rule: &mut QualifiedRule) {
  match &mut rule.prelude {
    QualifiedRulePrelude::SelectorList(list) => {
      for complex in &mut list.children {
        if complex_selector_contains_compiled_class(complex) {
          increase_complex_selector_specificity(complex);
        }
      }
    }
    QualifiedRulePrelude::RelativeSelectorList(list) => {
      for relative in &mut list.children {
        if complex_selector_contains_compiled_class(&relative.selector) {
          increase_complex_selector_specificity(&mut relative.selector);
        }
      }
    }
    _ => {}
  }
}

fn complex_selector_contains_compiled_class(selector: &ComplexSelector) -> bool {
  selector.children.iter().any(|child| match child {
    ComplexSelectorChildren::CompoundSelector(compound) => {
      compound_contains_compiled_class(compound)
    }
    ComplexSelectorChildren::Combinator(_) => false,
  })
}

fn compound_contains_compiled_class(compound: &CompoundSelector) -> bool {
  compound
    .subclass_selectors
    .iter()
    .any(|selector| match selector {
      SubclassSelector::Class(class_selector) => class_selector.text.value.starts_with('_'),
      SubclassSelector::PseudoClass(pseudo) => pseudo_class_contains_compiled_class(pseudo),
      SubclassSelector::PseudoElement(pseudo) => pseudo_element_contains_compiled_class(pseudo),
      _ => false,
    })
}

fn pseudo_class_contains_compiled_class(pseudo: &PseudoClassSelector) -> bool {
  pseudo
    .children
    .as_ref()
    .map(|children| {
      children.iter().any(|child| match child {
        PseudoClassSelectorChildren::ComplexSelector(selector) => {
          complex_selector_contains_compiled_class(selector)
        }
        PseudoClassSelectorChildren::SelectorList(list) => list
          .children
          .iter()
          .any(complex_selector_contains_compiled_class),
        PseudoClassSelectorChildren::RelativeSelectorList(list) => list
          .children
          .iter()
          .any(|relative| complex_selector_contains_compiled_class(&relative.selector)),
        PseudoClassSelectorChildren::CompoundSelectorList(list) => {
          list.children.iter().any(compound_contains_compiled_class)
        }
        PseudoClassSelectorChildren::ForgivingSelectorList(list) => {
          list.children.iter().any(|item| match item {
            swc_core::css::ast::ForgivingComplexSelector::ComplexSelector(selector) => {
              complex_selector_contains_compiled_class(selector)
            }
            swc_core::css::ast::ForgivingComplexSelector::ListOfComponentValues(_) => false,
          })
        }
        PseudoClassSelectorChildren::ForgivingRelativeSelectorList(list) => {
          list.children.iter().any(|item| match item {
            swc_core::css::ast::ForgivingRelativeSelector::RelativeSelector(relative) => {
              complex_selector_contains_compiled_class(&relative.selector)
            }
            swc_core::css::ast::ForgivingRelativeSelector::ListOfComponentValues(_) => false,
          })
        }
        _ => false,
      })
    })
    .unwrap_or(false)
}

fn pseudo_element_contains_compiled_class(pseudo: &PseudoElementSelector) -> bool {
  pseudo
    .children
    .as_ref()
    .map(|children| {
      children.iter().any(|child| match child {
        PseudoElementSelectorChildren::CompoundSelector(compound) => {
          compound_contains_compiled_class(compound)
        }
        _ => false,
      })
    })
    .unwrap_or(false)
}

fn increase_complex_selector_specificity(selector: &mut ComplexSelector) {
  for child in &mut selector.children {
    if let ComplexSelectorChildren::CompoundSelector(compound) = child {
      increase_compound_selector_specificity(compound);
    }
  }
}

fn increase_compound_selector_specificity(compound: &mut CompoundSelector) {
  let mut index = 0;

  while index < compound.subclass_selectors.len() {
    match &mut compound.subclass_selectors[index] {
      SubclassSelector::Class(_) => {
        compound
          .subclass_selectors
          .insert(index + 1, create_specificity_pseudo());
        index += 2;
      }
      SubclassSelector::PseudoClass(pseudo) => {
        increase_pseudo_class_specificity(pseudo);
        index += 1;
      }
      SubclassSelector::PseudoElement(pseudo) => {
        increase_pseudo_element_specificity(pseudo);
        index += 1;
      }
      _ => {
        index += 1;
      }
    }
  }
}

fn increase_pseudo_class_specificity(pseudo: &mut PseudoClassSelector) {
  if let Some(children) = &mut pseudo.children {
    for child in children {
      match child {
        PseudoClassSelectorChildren::ComplexSelector(selector) => {
          increase_complex_selector_specificity(selector)
        }
        PseudoClassSelectorChildren::SelectorList(list) => {
          for selector in &mut list.children {
            increase_complex_selector_specificity(selector);
          }
        }
        PseudoClassSelectorChildren::RelativeSelectorList(list) => {
          for relative in &mut list.children {
            increase_complex_selector_specificity(&mut relative.selector);
          }
        }
        PseudoClassSelectorChildren::CompoundSelectorList(list) => {
          for compound in &mut list.children {
            increase_compound_selector_specificity(compound);
          }
        }
        PseudoClassSelectorChildren::ForgivingSelectorList(list) => {
          for item in &mut list.children {
            if let swc_core::css::ast::ForgivingComplexSelector::ComplexSelector(selector) = item {
              increase_complex_selector_specificity(selector);
            }
          }
        }
        PseudoClassSelectorChildren::ForgivingRelativeSelectorList(list) => {
          for item in &mut list.children {
            if let swc_core::css::ast::ForgivingRelativeSelector::RelativeSelector(relative) = item
            {
              increase_complex_selector_specificity(&mut relative.selector);
            }
          }
        }
        _ => {}
      }
    }
  }
}

fn increase_pseudo_element_specificity(pseudo: &mut PseudoElementSelector) {
  if let Some(children) = &mut pseudo.children {
    for child in children {
      if let PseudoElementSelectorChildren::CompoundSelector(compound) = child {
        increase_compound_selector_specificity(compound);
      }
    }
  }
}

fn create_specificity_pseudo() -> SubclassSelector {
  let selector_list = SelectorList {
    span: DUMMY_SP,
    children: vec![ComplexSelector {
      span: DUMMY_SP,
      children: vec![ComplexSelectorChildren::CompoundSelector(
        CompoundSelector {
          span: DUMMY_SP,
          nesting_selector: None,
          type_selector: None,
          subclass_selectors: vec![SubclassSelector::Id(IdSelector {
            span: DUMMY_SP,
            text: create_escaped_hash_ident(),
          })],
        },
      )],
    }],
  };

  let pseudo = PseudoClassSelector {
    span: DUMMY_SP,
    name: create_ident("not"),
    children: Some(vec![PseudoClassSelectorChildren::SelectorList(
      selector_list,
    )]),
  };

  SubclassSelector::PseudoClass(pseudo)
}

fn create_escaped_hash_ident() -> swc_core::css::ast::Ident {
  swc_core::css::ast::Ident {
    span: DUMMY_SP,
    value: Atom::from("#"),
    raw: Some(Atom::from("\\#")),
  }
}

fn create_ident(value: &str) -> swc_core::css::ast::Ident {
  swc_core::css::ast::Ident {
    span: DUMMY_SP,
    value: Atom::from(value),
    raw: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use crate::utils_constants::INCREASE_SPECIFICITY_SELECTOR;
  use swc_core::common::{FileName, SourceMap, input::StringInput};
  use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
  use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

  fn parse_stylesheet(css: &str) -> Stylesheet {
    let cm: std::sync::Arc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.css".into()).into(), css.to_string());
    let mut errors = vec![];
    parse_string_input::<Stylesheet>(
      StringInput::from(&*fm),
      None,
      ParserConfig::default(),
      &mut errors,
    )
    .expect("failed to parse test stylesheet")
  }

  fn serialize_stylesheet(stylesheet: &Stylesheet) -> String {
    let mut output = String::new();
    {
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
      generator
        .emit(stylesheet)
        .expect("failed to serialize stylesheet");
    }
    output
  }

  #[test]
  fn ignores_non_prefixed_class_names() {
    let mut stylesheet = parse_stylesheet(".foo {}");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    IncreaseSpecificity.run(&mut stylesheet, &mut ctx);

    assert_eq!(serialize_stylesheet(&stylesheet), ".foo {}");
  }

  #[test]
  fn increases_specificity_of_compiled_classes() {
    let mut stylesheet = parse_stylesheet("._foo {}");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    IncreaseSpecificity.run(&mut stylesheet, &mut ctx);

    assert_eq!(
      serialize_stylesheet(&stylesheet),
      format!("._foo{} {{}}", INCREASE_SPECIFICITY_SELECTOR),
    );
  }

  #[test]
  fn handles_nested_pseudos() {
    let mut stylesheet =
      parse_stylesheet("._foo:hover { color: red; } ._bar::before { content: ''; }");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    IncreaseSpecificity.run(&mut stylesheet, &mut ctx);

    let serialized = serialize_stylesheet(&stylesheet);
    assert!(serialized.contains(&format!("._foo{}:hover", INCREASE_SPECIFICITY_SELECTOR)));
    assert!(serialized.contains(&format!("._bar{}::before", INCREASE_SPECIFICITY_SELECTOR)));
  }
}
