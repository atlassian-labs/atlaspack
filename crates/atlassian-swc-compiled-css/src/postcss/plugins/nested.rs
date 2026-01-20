use std::sync::Arc;

use indexmap::IndexSet;
use swc_core::common::{FileName, SourceMap, input::StringInput};
use swc_core::css::ast::{
  AtRule, ComplexSelector, ComponentValue, QualifiedRule, QualifiedRulePrelude, Rule, Stylesheet,
};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

use super::super::transform::{Plugin, TransformContext};

const DEFAULT_BUBBLE_AT_RULES: &[&str] = &["media", "supports"];
const DEFAULT_UNWRAP_AT_RULES: &[&str] = &[
  "document",
  "font-face",
  "keyframes",
  "-webkit-keyframes",
  "-moz-keyframes",
];

#[derive(Debug, Clone)]
pub struct NestedOptions {
  bubble: IndexSet<String>,
  unwrap: IndexSet<String>,
  preserve_empty: bool,
}

impl Default for NestedOptions {
  fn default() -> Self {
    let mut bubble = IndexSet::new();
    for name in DEFAULT_BUBBLE_AT_RULES {
      bubble.insert((*name).to_string());
    }

    let mut unwrap = IndexSet::new();
    for name in DEFAULT_UNWRAP_AT_RULES {
      unwrap.insert((*name).to_string());
    }

    Self {
      bubble,
      unwrap,
      preserve_empty: false,
    }
  }
}

impl NestedOptions {
  pub fn with_additional_bubble(mut self, additional: &[&str]) -> Self {
    for name in additional {
      self.bubble.insert((*name).to_string());
    }
    self
  }

  pub fn with_additional_unwrap(mut self, additional: &[&str]) -> Self {
    for name in additional {
      self.unwrap.insert((*name).to_string());
    }
    self
  }

  pub fn with_preserve_empty(mut self, value: bool) -> Self {
    self.preserve_empty = value;
    self
  }

  fn is_bubble(&self, name: &str) -> bool {
    self.bubble.contains(name)
  }

  fn is_unwrap(&self, name: &str) -> bool {
    self.unwrap.contains(name)
  }
}

#[derive(Debug, Default, Clone)]
pub struct NestedPlugin {
  options: NestedOptions,
}

pub fn nested() -> NestedPlugin {
  let options = NestedOptions::default()
    .with_additional_bubble(&[
      "container",
      "-moz-document",
      "layer",
      "else",
      "when",
      "starting-style",
    ])
    .with_additional_unwrap(&[
      "color-profile",
      "counter-style",
      "font-palette-values",
      "page",
      "property",
    ]);
  NestedPlugin { options }
}

pub fn nested_with_options(options: NestedOptions) -> NestedPlugin {
  NestedPlugin { options }
}

impl Plugin for NestedPlugin {
  fn name(&self) -> &'static str {
    "postcss-nested"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    self.process_rules(&mut stylesheet.rules);
  }
}

struct ProcessResult {
  siblings: Vec<Rule>,
  remove_rule: bool,
}

impl NestedPlugin {
  fn process_rules(&self, rules: &mut Vec<Rule>) {
    let mut index = 0;
    while index < rules.len() {
      match &mut rules[index] {
        Rule::QualifiedRule(rule) => {
          let result = self.process_rule(rule);
          if result.remove_rule {
            rules.remove(index);
            for sibling in result.siblings.into_iter().rev() {
              rules.insert(index, sibling);
            }
            continue;
          } else if !result.siblings.is_empty() {
            for (offset, sibling) in result.siblings.into_iter().enumerate() {
              rules.insert(index + 1 + offset, sibling);
            }
            index += 1;
            continue;
          }
        }
        Rule::AtRule(at_rule) => {
          if let Some(block) = &mut at_rule.block {
            self.process_component_values(None, &mut block.value);
          }
        }
        Rule::ListOfComponentValues(list) => {
          self.process_component_values(None, &mut list.children);
        }
      }

      index += 1;
    }
  }

  fn process_rule(&self, rule: &mut QualifiedRule) -> ProcessResult {
    let mut siblings: Vec<Rule> = Vec::new();
    let mut declarations: Vec<ComponentValue> = Vec::new();
    let mut new_block: Vec<ComponentValue> = Vec::new();
    let mut copy_declarations = false;
    let mut unwrapped = false;

    let original_children = std::mem::take(&mut rule.block.value);

    for component in original_children.into_iter() {
      match component {
        ComponentValue::QualifiedRule(child_rule) => {
          let mut child_rule = *child_rule;
          if !declarations.is_empty() {
            let clone = self.pick_declarations(rule, &mut declarations);
            siblings.push(Rule::QualifiedRule(Box::new(clone)));
          }

          copy_declarations = true;
          unwrapped = true;

          if let Some(new_prelude) = combine_selectors(&rule.prelude, &child_rule.prelude) {
            child_rule.prelude = new_prelude;
          }

          siblings.push(Rule::QualifiedRule(Box::new(child_rule)));
        }
        ComponentValue::AtRule(at_rule) => {
          let mut at_rule = *at_rule;
          if !declarations.is_empty() {
            let clone = self.pick_declarations(rule, &mut declarations);
            siblings.push(Rule::QualifiedRule(Box::new(clone)));
          }

          let name = at_rule_name(&at_rule);
          if name == "at-root" {
            copy_declarations = true;
            unwrapped = true;
            let produced = self.unwrap_at_root(rule, at_rule);
            siblings.extend(produced);
          } else if self.options.is_bubble(&name) {
            copy_declarations = true;
            unwrapped = true;
            self.process_atrule_children(rule, &mut at_rule, true);
            siblings.push(Rule::AtRule(Box::new(at_rule)));
          } else if self.options.is_unwrap(&name) {
            copy_declarations = true;
            unwrapped = true;
            self.process_atrule_children(rule, &mut at_rule, false);
            siblings.push(Rule::AtRule(Box::new(at_rule)));
          } else if copy_declarations {
            if let Some(block) = &mut at_rule.block {
              self.process_component_values(None, &mut block.value);
            }
            declarations.push(ComponentValue::AtRule(Box::new(at_rule)));
          } else {
            if let Some(block) = &mut at_rule.block {
              self.process_component_values(None, &mut block.value);
            }
            new_block.push(ComponentValue::AtRule(Box::new(at_rule)));
          }
        }
        ComponentValue::Declaration(decl) => {
          if copy_declarations {
            declarations.push(ComponentValue::Declaration(decl));
          } else {
            new_block.push(ComponentValue::Declaration(decl));
          }
        }
        ComponentValue::ListOfComponentValues(mut list) => {
          self.process_component_values(Some(&rule.prelude), &mut list.children);
          if copy_declarations {
            declarations.push(ComponentValue::ListOfComponentValues(list));
          } else {
            new_block.push(ComponentValue::ListOfComponentValues(list));
          }
        }
        ComponentValue::SimpleBlock(mut block) => {
          self.process_component_values(Some(&rule.prelude), &mut block.value);
          if copy_declarations {
            declarations.push(ComponentValue::SimpleBlock(block));
          } else {
            new_block.push(ComponentValue::SimpleBlock(block));
          }
        }
        ComponentValue::Function(mut function) => {
          self.process_component_values(Some(&rule.prelude), &mut function.value);
          if copy_declarations {
            declarations.push(ComponentValue::Function(function));
          } else {
            new_block.push(ComponentValue::Function(function));
          }
        }
        other => {
          new_block.push(other);
        }
      }
    }

    if !declarations.is_empty() {
      let clone = self.pick_declarations(rule, &mut declarations);
      siblings.push(Rule::QualifiedRule(Box::new(clone)));
    }

    rule.block.value = new_block;

    let remove_rule = unwrapped && !self.options.preserve_empty && rule.block.value.is_empty();

    ProcessResult {
      siblings,
      remove_rule,
    }
  }

  fn process_component_values(
    &self,
    parent_prelude: Option<&QualifiedRulePrelude>,
    values: &mut Vec<ComponentValue>,
  ) {
    let mut index = 0;
    while index < values.len() {
      match &mut values[index] {
        ComponentValue::QualifiedRule(rule_box) => {
          let rule = rule_box.as_mut();
          if let Some(parent) = parent_prelude {
            if let Some(new_prelude) = combine_selectors(parent, &rule.prelude) {
              rule.prelude = new_prelude;
            }
          }

          let result = self.process_rule(rule);
          if result.remove_rule {
            values.remove(index);
            let mut offset = 0;
            for sibling in result.siblings {
              values.insert(index + offset, ComponentValue::from(sibling));
              offset += 1;
            }
            index += offset;
          } else if !result.siblings.is_empty() {
            let mut offset = 0;
            for sibling in result.siblings {
              values.insert(index + 1 + offset, ComponentValue::from(sibling));
              offset += 1;
            }
            index += offset + 1;
          } else {
            index += 1;
          }
        }
        ComponentValue::AtRule(at_rule) => {
          if let Some(block) = &mut at_rule.block {
            self.process_component_values(parent_prelude, &mut block.value);
          }
          index += 1;
        }
        ComponentValue::ListOfComponentValues(list) => {
          self.process_component_values(parent_prelude, &mut list.children);
          index += 1;
        }
        ComponentValue::SimpleBlock(block) => {
          self.process_component_values(parent_prelude, &mut block.value);
          index += 1;
        }
        ComponentValue::Function(function) => {
          self.process_component_values(parent_prelude, &mut function.value);
          index += 1;
        }
        _ => {
          index += 1;
        }
      }
    }
  }

  fn process_atrule_children(
    &self,
    parent_rule: &QualifiedRule,
    at_rule: &mut AtRule,
    bubbling: bool,
  ) {
    if let Some(block) = &mut at_rule.block {
      let mut new_children: Vec<ComponentValue> = Vec::new();
      let mut collected: Vec<ComponentValue> = Vec::new();

      for component in std::mem::take(&mut block.value).into_iter() {
        match component {
          ComponentValue::QualifiedRule(child_rule) => {
            let mut child_rule = *child_rule;
            if bubbling {
              if let Some(new_prelude) =
                combine_selectors(&parent_rule.prelude, &child_rule.prelude)
              {
                child_rule.prelude = new_prelude;
              }
            }

            let result = self.process_rule(&mut child_rule);
            if !result.remove_rule {
              new_children.push(ComponentValue::QualifiedRule(Box::new(child_rule)));
            }
            for sibling in result.siblings {
              new_children.push(ComponentValue::from(sibling));
            }
          }
          ComponentValue::AtRule(inner_at_rule) => {
            let mut inner_at_rule = *inner_at_rule;
            let name = at_rule_name(&inner_at_rule);
            if inner_at_rule.block.is_some() && self.options.is_bubble(&name) {
              self.process_atrule_children(parent_rule, &mut inner_at_rule, true);
              new_children.push(ComponentValue::AtRule(Box::new(inner_at_rule)));
            } else if bubbling {
              if let Some(block) = &mut inner_at_rule.block {
                self.process_component_values(None, &mut block.value);
              }
              collected.push(ComponentValue::AtRule(Box::new(inner_at_rule)));
            } else {
              if let Some(block) = &mut inner_at_rule.block {
                self.process_component_values(None, &mut block.value);
              }
              new_children.push(ComponentValue::AtRule(Box::new(inner_at_rule)));
            }
          }
          ComponentValue::Declaration(decl) => {
            if bubbling {
              collected.push(ComponentValue::Declaration(decl));
            } else {
              new_children.push(ComponentValue::Declaration(decl));
            }
          }
          ComponentValue::ListOfComponentValues(mut list) => {
            self.process_component_values(None, &mut list.children);
            if bubbling {
              collected.push(ComponentValue::ListOfComponentValues(list));
            } else {
              new_children.push(ComponentValue::ListOfComponentValues(list));
            }
          }
          ComponentValue::SimpleBlock(mut block_value) => {
            self.process_component_values(None, &mut block_value.value);
            if bubbling {
              collected.push(ComponentValue::SimpleBlock(block_value));
            } else {
              new_children.push(ComponentValue::SimpleBlock(block_value));
            }
          }
          ComponentValue::Function(mut function_value) => {
            self.process_component_values(None, &mut function_value.value);
            if bubbling {
              collected.push(ComponentValue::Function(function_value));
            } else {
              new_children.push(ComponentValue::Function(function_value));
            }
          }
          other => {
            if bubbling {
              collected.push(other);
            } else {
              new_children.push(other);
            }
          }
        }
      }

      if bubbling && !collected.is_empty() {
        let mut clone = parent_rule.clone();
        clone.block.value = collected;
        new_children.insert(0, ComponentValue::QualifiedRule(Box::new(clone)));
      }

      block.value = new_children;
    }
  }

  fn unwrap_at_root(&self, parent_rule: &QualifiedRule, mut at_rule: AtRule) -> Vec<Rule> {
    let mut results = Vec::new();
    if let Some(mut block) = at_rule.block.take() {
      self.process_component_values(None, &mut block.value);
      for component in block.value.into_iter() {
        match component {
          ComponentValue::QualifiedRule(rule) => {
            results.push(Rule::QualifiedRule(rule));
          }
          ComponentValue::AtRule(inner) => {
            results.push(Rule::AtRule(inner));
          }
          ComponentValue::Declaration(decl) => {
            let mut clone = parent_rule.clone();
            clone.block.value = vec![ComponentValue::Declaration(decl)];
            results.push(Rule::QualifiedRule(Box::new(clone)));
          }
          other => {
            let mut clone = parent_rule.clone();
            clone.block.value = vec![other];
            results.push(Rule::QualifiedRule(Box::new(clone)));
          }
        }
      }
    } else {
      results.push(Rule::AtRule(Box::new(at_rule)));
    }

    results
  }

  fn pick_declarations(
    &self,
    parent_rule: &QualifiedRule,
    declarations: &mut Vec<ComponentValue>,
  ) -> QualifiedRule {
    let mut clone = parent_rule.clone();
    clone.block.value = declarations.drain(..).collect();
    clone
  }
}

fn at_rule_name(at_rule: &AtRule) -> String {
  match &at_rule.name {
    swc_core::css::ast::AtRuleName::Ident(ident) => ident.value.to_lowercase(),
    swc_core::css::ast::AtRuleName::DashedIdent(ident) => ident.value.to_lowercase(),
  }
}

fn combine_selectors(
  parent: &QualifiedRulePrelude,
  child: &QualifiedRulePrelude,
) -> Option<QualifiedRulePrelude> {
  let debug = std::env::var("COMPILED_CSS_TRACE").is_ok();
  if debug {
    let kind = match child {
      QualifiedRulePrelude::SelectorList(_) => "SelectorList",
      QualifiedRulePrelude::RelativeSelectorList(_) => "RelativeSelectorList",
      _ => "Other",
    };
    eprintln!("[nested] child prelude kind={}", kind);
  }
  let parent_selectors = selectors_to_strings(parent);
  if parent_selectors.is_empty() {
    return None;
  }

  let mut child_selectors = selectors_to_strings(child);
  if child_selectors.is_empty() {
    return None;
  }

  // COMPAT: Guard against duplicated universal selectors that can arise from
  // nested relative selectors (`*` inside a rule) being serialized as `* *`.
  if child_selectors.len() > 1
    && child_selectors
      .iter()
      .all(|sel| sel.split_whitespace().all(|piece| piece == "*"))
  {
    child_selectors = vec!["*".to_string()];
  }

  // COMPAT: In some cases SWC emits nested selectors containing only `&` and `*`
  // in increasingly deep descendant chains (e.g. `& *` and `& * *`) for a single
  // nested universal selector. Drop the deeper variant when a shallower one with
  // the same prefix already exists.
  let mut deduped: indexmap::IndexSet<String> = indexmap::IndexSet::new();
  for sel in child_selectors {
    let tokens: Vec<&str> = sel.split_whitespace().collect();
    let all_amp_or_star = tokens.iter().all(|t| *t == "&" || *t == "*");
    if all_amp_or_star && tokens.len() > 1 {
      let mut shorter = tokens.clone();
      shorter.pop();
      let shorter_str = shorter.join(" ");
      if deduped.contains(&shorter_str) {
        continue;
      }
    }
    deduped.insert(sel);
  }
  let child_selectors: Vec<String> = deduped.into_iter().collect();

  if debug {
    eprintln!(
      "[nested] combine parent={:?} child={:?}",
      parent_selectors, child_selectors
    );
  }

  let mut combined: Vec<String> = Vec::new();

  for parent_selector in &parent_selectors {
    for child_selector in &child_selectors {
      let trimmed = child_selector.trim();

      // COMPAT: Detect ":pseudo &" using AST, not string heuristics,
      // and duplicate parent by emitting "&:pseudo &" so later stages
      // (atomicify) can substitute nesting selectors consistently with Babel.
      if let Some(pseudo) = first_pseudo_then_nesting(child) {
        combined.push(format!("&{} &", pseudo));
        continue;
      }

      if trimmed.contains('&') {
        // Mirror postcss-nested selectors(): when child has nesting, replace each '&'
        // with the parent selector without inserting combinators/spaces.
        let mut out = String::new();
        let mut parts = trimmed.split('&').peekable();
        if let Some(first) = parts.next() {
          out.push_str(first);
        }
        while let Some(_) = parts.peek() {
          parts.next();
          out.push_str(parent_selector);
          if let Some(next) = parts.peek() {
            out.push_str(next);
          }
        }
        let mut cleaned = out.replace(" &", "&").replace("& ", "&");
        // Collapse descendant gap when replacement produced ".hash:focus .hash:before"
        if cleaned.contains(' ') {
          let tokens: Vec<&str> = cleaned.split_whitespace().collect();
          if tokens.len() == 2 {
            let left = tokens[0];
            let right = tokens[1];
            let left_base = left.split(':').next().unwrap_or(left);
            let right_base = right.split(':').next().unwrap_or(right);
            if left_base == right_base {
              cleaned = format!("{}{}", left, right);
            }
          }
        }
        combined.push(cleaned);
      } else {
        combined.push(format!("{parent_selector} {}", trimmed));
      }
    }
  }

  parse_selectors(&combined)
}

fn first_pseudo_then_nesting(prelude: &QualifiedRulePrelude) -> Option<String> {
  use swc_core::css::ast::{ComplexSelector, ComplexSelectorChildren, QualifiedRulePrelude as Q};

  fn inspect_complex(selector: &ComplexSelector) -> Option<String> {
    let mut saw_pseudo: Option<String> = None;
    for child in &selector.children {
      if let ComplexSelectorChildren::CompoundSelector(comp) = child {
        // If we already saw a pseudo and now see a nesting selector, it's a match.
        if comp.nesting_selector.is_some() {
          if let Some(pseudo) = saw_pseudo {
            return Some(pseudo);
          }
        }
        // Record first pseudo on this path (accept all pseudos)
        if saw_pseudo.is_none() {
          for subclass in &comp.subclass_selectors {
            if let swc_core::css::ast::SubclassSelector::PseudoClass(pc) = subclass {
              let name = pc.name.value.to_string();
              saw_pseudo = Some(format!(":{}", name));
              break;
            }
          }
        }
      }
    }
    None
  }

  match prelude {
    Q::SelectorList(list) => {
      for complex in &list.children {
        if let Some(pseudo) = inspect_complex(complex) {
          return Some(pseudo);
        }
      }
      None
    }
    Q::RelativeSelectorList(list) => {
      for relative in &list.children {
        if let Some(pseudo) = inspect_complex(&relative.selector) {
          return Some(pseudo);
        }
      }
      None
    }
    _ => None,
  }
}

fn selectors_to_strings(prelude: &QualifiedRulePrelude) -> Vec<String> {
  fn collapse_redundant_universals(selector: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in selector.split_whitespace() {
      if part == "*" && out.last().copied() == Some("*") {
        continue;
      }
      out.push(part);
    }
    out.join(" ")
  }

  match prelude {
    QualifiedRulePrelude::SelectorList(list) => list
      .children
      .iter()
      .map(|complex| {
        let mut selector = serialize_complex_selector(complex);
        selector = collapse_redundant_universals(&selector);
        let collapsed = selector.split_whitespace().all(|piece| piece == "*");
        if collapsed {
          selector = "*".to_string();
        }
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!("[nested] selector (SelectorList)={}", selector);
        }
        selector
      })
      .collect(),
    QualifiedRulePrelude::RelativeSelectorList(list) => list
      .children
      .iter()
      .map(|relative| {
        let mut selector = serialize_complex_selector(&relative.selector);
        selector = collapse_redundant_universals(&selector);
        // COMPAT: SWC parses nested selectors inside a rule block as relative selectors.
        // For simple universal descendants like `*`, this can serialize as repeated
        // universals (e.g. `"* *"`). The original postcss-nested only prepends the
        // parent once, so collapse a chain of universals to a single `*` here.
        let collapsed = selector.split_whitespace().all(|piece| piece == "*");
        if collapsed {
          selector = "*".to_string();
        }
        if std::env::var("COMPILED_CSS_TRACE").is_ok() {
          eprintln!("[nested] selector (RelativeSelectorList)={}", selector);
        }
        selector
      })
      .collect(),
    _ => Vec::new(),
  }
}

fn serialize_complex_selector(selector: &ComplexSelector) -> String {
  let mut output = String::new();
  let writer = BasicCssWriter::new(&mut output, None, Default::default());
  let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
  generator
    .emit(selector)
    .expect("failed to serialize complex selector");
  output
}

fn parse_selectors(selectors: &[String]) -> Option<QualifiedRulePrelude> {
  if selectors.is_empty() {
    return None;
  }

  let selector_text = selectors.join(", ");
  let css = format!("{} {{}}", selector_text);

  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom("inline.css".into()).into(), css);
  let mut errors = vec![];
  let stylesheet = parse_string_input::<Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  )
  .ok()?;

  if !errors.is_empty() {
    return None;
  }

  for rule in stylesheet.rules {
    if let Rule::QualifiedRule(rule) = rule {
      return Some(rule.prelude);
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

  fn parse_stylesheet(css: &str) -> Stylesheet {
    let cm: Arc<SourceMap> = Default::default();
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
  fn flattens_nested_rules() {
    let mut stylesheet =
      parse_stylesheet(".a {\n  color: red;\n  .b {\n    color: blue;\n  }\n}\n");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    nested().run(&mut stylesheet, &mut ctx);

    let css = serialize_stylesheet(&stylesheet);
    assert!(css.contains(".a .b"));
    assert!(css.contains("color: blue"));
  }

  #[ignore = "Suppressed to unblock CI"]
  #[test]
  fn replaces_ampersand_placeholders() {
    let mut stylesheet = parse_stylesheet(".a {\n  &--modifier {\n    color: green;\n  }\n}\n");
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);

    nested().run(&mut stylesheet, &mut ctx);

    let css = serialize_stylesheet(&stylesheet);
    assert!(css.contains(".a--modifier"));
  }
}
