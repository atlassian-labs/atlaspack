use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use oxc_browserslist::{Opts, execute};
use swc_core::css::ast::{ComponentValue, Declaration, Rule, Stylesheet};

use super::super::transform::{Plugin, TransformContext};
use crate::postcss::plugins::expand_shorthands::types::{
  declaration_property_name, parse_value_to_components, serialize_component_values,
};

static FROM_INITIAL: Lazy<HashMap<String, String>> = Lazy::new(|| {
  serde_json::from_str(include_str!("data/fromInitial.json"))
    .expect("failed to parse fromInitial.json")
});

static TO_INITIAL: Lazy<HashMap<String, String>> = Lazy::new(|| {
  serde_json::from_str(include_str!("data/toInitial.json")).expect("failed to parse toInitial.json")
});

static DEFAULT_IGNORE_PROPS: Lazy<HashSet<String>> = Lazy::new(|| {
  let values: Vec<String> = serde_json::from_str(include_str!("lib/ignoreProps.json"))
    .expect("failed to parse ignoreProps.json");
  values
    .into_iter()
    .map(|value| value.to_ascii_lowercase())
    .collect()
});

static CSS_INITIAL_VALUE_SUPPORT: Lazy<HashMap<String, HashSet<String>>> = Lazy::new(|| {
  let raw: HashMap<String, HashMap<String, String>> =
    serde_json::from_str(include_str!("data/css-initial-value-stats.json"))
      .expect("failed to parse css-initial-value-stats.json");

  raw
    .into_iter()
    .map(|(browser, versions)| {
      let supported_versions = versions
        .into_iter()
        .filter_map(|(version, status)| {
          if status == "y" {
            Some(version.to_ascii_lowercase())
          } else {
            None
          }
        })
        .collect::<HashSet<_>>();
      (browser.to_ascii_lowercase(), supported_versions)
    })
    .collect()
});

#[derive(Debug, Clone)]
pub struct ReduceInitial {
  initial_support: bool,
  ignore_props: HashSet<String>,
}

impl ReduceInitial {
  fn new() -> Self {
    Self {
      initial_support: detect_initial_support(),
      ignore_props: DEFAULT_IGNORE_PROPS.clone(),
    }
  }

  fn process_stylesheet(&self, stylesheet: &mut Stylesheet) {
    for rule in &mut stylesheet.rules {
      self.process_rule(rule);
    }
  }

  fn process_rule(&self, rule: &mut Rule) {
    match rule {
      Rule::QualifiedRule(rule) => {
        self.process_component_values(&mut rule.block.value);
      }
      Rule::AtRule(at_rule) => {
        if let Some(block) = &mut at_rule.block {
          self.process_component_values(&mut block.value);
        }
      }
      Rule::ListOfComponentValues(list) => {
        self.process_component_values(&mut list.children);
      }
    }
  }

  fn process_component_values(&self, values: &mut [ComponentValue]) {
    for value in values {
      match value {
        ComponentValue::Declaration(declaration) => {
          self.process_declaration(declaration);
        }
        ComponentValue::QualifiedRule(rule) => {
          self.process_component_values(&mut rule.block.value);
        }
        ComponentValue::AtRule(at_rule) => {
          if let Some(block) = &mut at_rule.block {
            self.process_component_values(&mut block.value);
          }
        }
        ComponentValue::SimpleBlock(block) => {
          self.process_component_values(&mut block.value);
        }
        ComponentValue::ListOfComponentValues(list) => {
          self.process_component_values(&mut list.children);
        }
        ComponentValue::Function(function) => {
          self.process_component_values(&mut function.value);
        }
        ComponentValue::KeyframeBlock(block) => {
          self.process_component_values(&mut block.block.value);
        }
        _ => {}
      }
    }
  }

  fn process_declaration(&self, declaration: &mut Declaration) {
    let property = declaration_property_name(&declaration.name);
    let normalized_property = property.to_ascii_lowercase();

    if self.ignore_props.contains(&normalized_property) {
      return;
    }

    let Some(serialized_value) = serialize_component_values(&declaration.value) else {
      return;
    };
    let normalized_value = serialized_value.to_ascii_lowercase();

    if self.initial_support {
      if let Some(target) = TO_INITIAL.get(&normalized_property) {
        if normalized_value == *target {
          if normalized_value != "initial" {
            declaration.value = parse_value_to_components("initial");
          }
          return;
        }
      }
    }

    if normalized_value != "initial" {
      return;
    }

    if let Some(fallback) = FROM_INITIAL.get(&normalized_property) {
      if serialized_value != *fallback {
        declaration.value = parse_value_to_components(fallback);
      }
    }
  }
}

impl Plugin for ReduceInitial {
  fn name(&self) -> &'static str {
    "postcss-reduce-initial"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    self.process_stylesheet(stylesheet);
  }
}

pub fn reduce_initial() -> ReduceInitial {
  ReduceInitial::new()
}

fn detect_initial_support() -> bool {
  let mut opts = Opts::default();
  opts.path = Some(env!("CARGO_MANIFEST_DIR").to_string());

  execute(&opts)
    .map(|entries| {
      entries.into_iter().all(|entry| {
        let browser = entry.name().to_ascii_lowercase();
        let version = entry.version().to_ascii_lowercase();
        css_initial_supported(&browser, &version)
      })
    })
    .unwrap_or(false)
}

fn css_initial_supported(browser: &str, version: &str) -> bool {
  CSS_INITIAL_VALUE_SUPPORT
    .get(browser)
    .map(|versions| versions.contains(version))
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::transform::{TransformContext, TransformCssOptions};
  use swc_core::common::{FileName, SourceMap, input::StringInput};
  use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
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
    .expect("failed to parse stylesheet")
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

  fn run_plugin(css: &str, plugin: &ReduceInitial) -> String {
    let mut stylesheet = parse_stylesheet(css);
    let options = TransformCssOptions::default();
    let mut ctx = TransformContext::new(&options);
    plugin.run(&mut stylesheet, &mut ctx);
    serialize_stylesheet(&stylesheet)
  }

  #[test]
  fn converts_known_values_to_initial_when_supported() {
    let plugin = ReduceInitial {
      initial_support: true,
      ignore_props: DEFAULT_IGNORE_PROPS.clone(),
    };
    let result = run_plugin(".a { background-color: transparent; }", &plugin);
    assert_eq!(result, ".a {\n  background-color: initial;\n}");
  }

  #[test]
  fn keeps_values_when_support_missing() {
    let plugin = ReduceInitial {
      initial_support: false,
      ignore_props: DEFAULT_IGNORE_PROPS.clone(),
    };
    let result = run_plugin(".a { background-color: transparent; }", &plugin);
    assert_eq!(result, ".a {\n  background-color: transparent;\n}");
  }

  #[test]
  fn replaces_initial_with_longhand_equivalent() {
    let plugin = ReduceInitial {
      initial_support: false,
      ignore_props: DEFAULT_IGNORE_PROPS.clone(),
    };
    let result = run_plugin(".a { border-top-width: initial; }", &plugin);
    assert_eq!(result, ".a {\n  border-top-width: medium;\n}");
  }

  #[test]
  fn ignores_configured_properties() {
    let mut ignore = DEFAULT_IGNORE_PROPS.clone();
    ignore.insert("background-color".into());
    let plugin = ReduceInitial {
      initial_support: true,
      ignore_props: ignore,
    };
    let result = run_plugin(".a { background-color: transparent; }", &plugin);
    assert_eq!(result, ".a {\n  background-color: transparent;\n}");
  }
}
