use std::sync::Arc;

use swc_core::common::{FileName, SourceMap, input::StringInput};
use swc_core::css::ast::Stylesheet;
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

use super::plugins::{
  discard_duplicates::discard_duplicates, merge_duplicate_at_rules::merge_duplicate_at_rules,
  sort_atomic_style_sheet as sort_plugin,
};
use super::transform::{Plugin, TransformContext, TransformCssOptions};

#[derive(Debug, Clone, Default)]
pub struct SortOptions {
  pub sort_at_rules: Option<bool>,
  pub sort_shorthand: Option<bool>,
}

pub fn sort_atomic_style_sheet(css: &str, options: SortOptions) -> String {
  let mut stylesheet = match parse_stylesheet(css) {
    Ok(sheet) => sheet,
    Err(_) => return css.to_string(),
  };

  let mut transform_options = TransformCssOptions::default();
  transform_options.sort_at_rules = options.sort_at_rules;
  transform_options.sort_shorthand = options.sort_shorthand;
  let mut ctx = TransformContext::new(&transform_options);

  discard_duplicates().run(&mut stylesheet, &mut ctx);
  merge_duplicate_at_rules().run(&mut stylesheet, &mut ctx);
  sort_plugin::sort_atomic_style_sheet(options.sort_at_rules, options.sort_shorthand)
    .run(&mut stylesheet, &mut ctx);

  match serialize_stylesheet(&stylesheet) {
    Ok(serialized) => serialized,
    Err(_) => css.to_string(),
  }
}

fn parse_stylesheet(css: &str) -> Result<Stylesheet, ()> {
  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(
    FileName::Custom("inline.css".into()).into(),
    css.to_string(),
  );
  let mut errors = vec![];

  match parse_string_input::<Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  ) {
    Ok(stylesheet) if errors.is_empty() => Ok(stylesheet),
    _ => Err(()),
  }
}

fn serialize_stylesheet(stylesheet: &Stylesheet) -> Result<String, ()> {
  let mut serialized_rules = Vec::with_capacity(stylesheet.rules.len());

  for rule in &stylesheet.rules {
    let mut output = String::new();
    {
      let writer = BasicCssWriter::new(&mut output, None, Default::default());
      let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: true });
      generator.emit(rule).map_err(|_| ())?;
    }
    serialized_rules.push(output);
  }

  Ok(serialized_rules.join("\n"))
}
