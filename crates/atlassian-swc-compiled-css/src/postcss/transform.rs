use std::{collections::HashMap, path::PathBuf, sync::Arc};

use indexmap::IndexSet;
use swc_core::common::{FileName, SourceMap, input::StringInput};
use swc_core::css::ast::Stylesheet;
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

#[cfg(feature = "postcss_engine")]
use super::postcss_pipeline::transform_css_via_postcss;

use super::plugins::discard_comments::collect_preserved_comments;
use super::plugins::atomicify_rules::HashStrategy;
use super::plugins::{
  atomicify_rules::atomicify_rules, discard_duplicates::discard_duplicates,
  discard_empty_rules::discard_empty_rules, expand_shorthands::index::expand_shorthands,
  extract_stylesheets::extract_stylesheets, flatten_multiple_selectors::flatten_multiple_selectors,
  increase_specificity::increase_specificity, nested::nested, normalize_css::normalize_css,
  normalize_whitespace::normalize_whitespace, parent_orphaned_pseudos::parent_orphaned_pseudos,
  sort_atomic_style_sheet::sort_atomic_style_sheet,
};

/// Options equivalent to `packages/css/src/transform.ts`.
#[derive(Debug, Clone, Default)]
pub struct TransformCssOptions {
  pub optimize_css: Option<bool>,
  pub class_name_compression_map: Option<HashMap<String, String>>,
  pub increase_specificity: Option<bool>,
  pub sort_at_rules: Option<bool>,
  pub sort_shorthand: Option<bool>,
  pub class_hash_prefix: Option<String>,
  pub flatten_multiple_selectors: Option<bool>,
  /// Controls the hash strategy used for atomic class name generation.
  /// @experimental Not part of the public API. May change without notice.
  pub hash_strategy: Option<HashStrategy>,
  pub declaration_placeholder: Option<String>,
  /// Path used to resolve the browserslist config for autoprefixer.
  /// Defaults to `cwd`, matching Babel's autoprefixer which uses `{ from: undefined }`.
  pub browserslist_config_path: Option<PathBuf>,
  /// Browserslist environment (e.g. "development" or "production") for package.json "browserslist".
  pub browserslist_env: Option<String>,
  /// Path used to resolve the browserslist config for cssnano plugins
  /// (reduce-initial, colormin, etc.).  Always resolves from the
  /// `@compiled/css` package directory (falling back to browserslist
  /// defaults), matching Babel's cssnano plugins which use `{ path: __dirname }`.
  pub cssnano_browserslist_config_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransformCssResult {
  pub sheets: Vec<String>,
  pub class_names: Vec<String>,
}

#[derive(Debug)]
pub struct CssTransformError {
  message: String,
}

impl CssTransformError {
  pub(crate) fn from_message(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }

  fn parser_error(css: &str, message: impl Into<String>) -> Self {
    let mut formatted = String::new();
    formatted.push_str(
      "An unhandled exception was raised when parsing your CSS, this is probably a bug!\n",
    );
    formatted.push_str(
            "Raise an issue here: https://github.com/atlassian-labs/compiled/issues/new?assignees=&labels=&template=bug_report.md&title=CSS%20Parsing%20Exception:\n\n",
        );
    formatted.push_str("Input CSS: {\n");
    formatted.push_str(css);
    formatted.push_str("\n}\n\n");
    formatted.push_str("Exception: ");
    formatted.push_str(&message.into());

    Self { message: formatted }
  }
}

impl std::fmt::Display for CssTransformError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.message)
  }
}

impl std::error::Error for CssTransformError {}

/// Execution context shared across plugins.
#[derive(Debug)]
pub struct TransformContext<'a> {
  pub options: &'a TransformCssOptions,
  pub sheets: Vec<String>,
  preserved_comments: Vec<String>,
  class_names: IndexSet<String>,
}

impl<'a> TransformContext<'a> {
  pub fn new(options: &'a TransformCssOptions) -> Self {
    Self {
      options,
      sheets: Vec::new(),
      preserved_comments: Vec::new(),
      class_names: IndexSet::new(),
    }
  }

  pub fn push_class_name(&mut self, class_name: impl Into<String>) {
    self.class_names.insert(class_name.into());
  }

  pub fn push_sheet(&mut self, sheet: impl Into<String>) {
    let raw = sheet.into();
    let normalized =
      crate::postcss::plugins::extract_stylesheets::normalize_block_value_spacing(&raw);
    self.sheets.push(normalized);
  }

  pub fn set_preserved_comments(&mut self, comments: Vec<String>) {
    self.preserved_comments = comments;
  }

  pub fn take_preserved_comments(&mut self) -> Vec<String> {
    std::mem::take(&mut self.preserved_comments)
  }

  fn first_class_from_sheet(sheet: &str) -> Option<String> {
    // Find first '.' and read until '{', whitespace, or comma.
    let dot = sheet.find('.')?;
    let rest = &sheet[dot + 1..];
    let end = rest
      .find(|c: char| c == '{' || c == ' ' || c == ',')
      .unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
      None
    } else {
      Some(name.to_string())
    }
  }

  fn reorder_class_names_by_sheets(mut class_names: Vec<String>, sheets: &[String]) -> Vec<String> {
    use std::collections::HashSet;
    let mut ordered_keys: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for sheet in sheets {
      if let Some(class_name) = Self::first_class_from_sheet(sheet) {
        if seen.insert(class_name.clone()) {
          ordered_keys.push(class_name);
        }
      }
    }

    if ordered_keys.is_empty() {
      return class_names;
    }

    let mut included: HashSet<String> = HashSet::new();
    let mut result: Vec<String> = Vec::with_capacity(class_names.len());

    // Add in the order classes appear in sheets
    for key in ordered_keys {
      if let Some(pos) = class_names.iter().position(|c| c == &key) {
        let name = class_names.remove(pos);
        included.insert(name.clone());
        result.push(name);
      }
    }

    // Append any remaining classes preserving original encounter order
    for name in class_names.into_iter() {
      if !included.contains(&name) {
        result.push(name);
      }
    }

    result
  }

  pub fn finish(self) -> TransformCssResult {
    let sheets: Vec<String> = self
      .sheets
      .into_iter()
      .map(|mut sheet| {
        sheet = sheet.replace(" *", "*");
        sheet = sheet.replace("* ", "*");
        sheet = sheet.replace("*-", "* -");
        sheet = sheet.replace("*+", "* +");
        sheet
      })
      .collect();
    let encountered: Vec<String> = self.class_names.into_iter().collect();
    let class_names = Self::reorder_class_names_by_sheets(encountered, &sheets);
    TransformCssResult {
      sheets,
      class_names,
    }
  }
}

/// Trait implemented by every plugin translation.
pub trait Plugin {
  fn name(&self) -> &'static str;
  fn run(&self, stylesheet: &mut Stylesheet, ctx: &mut TransformContext<'_>);
}

/// Parse CSS source into an AST using swc's CSS parser.
fn parse_stylesheet(css: &str) -> Result<Stylesheet, CssTransformError> {
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
    Ok(stylesheet) => {
      if let Some(error) = errors.into_iter().next() {
        let message = format!("{error:?}");
        Err(CssTransformError::parser_error(css, message))
      } else {
        Ok(stylesheet)
      }
    }
    Err(err) => Err(CssTransformError::parser_error(css, format!("{err:?}"))),
  }
}

/// Serialize a stylesheet back to CSS text.
fn serialize_stylesheet(stylesheet: &Stylesheet) -> Result<String, CssTransformError> {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
    generator.emit(stylesheet).map_err(|err| {
      CssTransformError::from_message(format!("failed to serialize stylesheet: {err}"))
    })?;
  }
  Ok(output)
}

/// Execute the CSS pipeline.
pub(crate) fn transform_css_via_swc_pipeline(
  css: &str,
  mut options: TransformCssOptions,
) -> Result<TransformCssResult, CssTransformError> {
  let preserved_comments = collect_preserved_comments(css, options.optimize_css);
  let mut stylesheet = match parse_stylesheet(css) {
    Ok(sheet) => sheet,
    Err(original_err) => {
      const PLACEHOLDER: &str = "__compiled_declaration_wrapper__";
      let wrapped = format!(".{PLACEHOLDER} {{{}}}", css);
      match parse_stylesheet(&wrapped) {
        Ok(sheet) => {
          options.declaration_placeholder = Some(format!(".{PLACEHOLDER}"));
          sheet
        }
        Err(_) => return Err(original_err),
      }
    }
  };
  let mut ctx = TransformContext::new(&options);
  ctx.set_preserved_comments(preserved_comments);

  let flatten_multiple_selectors_option = options.flatten_multiple_selectors.unwrap_or(true);

  let mut pipeline: Vec<Box<dyn Plugin>> = Vec::new();
  pipeline.push(Box::new(discard_duplicates()));
  pipeline.push(Box::new(discard_empty_rules()));
  pipeline.push(Box::new(parent_orphaned_pseudos()));
  pipeline.push(Box::new(nested()));

  // Expand shorthands early so longhands (e.g., background-color) flow through
  // reduce-initial/colormin before hashing, matching the Babel plugin order.
  pipeline.push(Box::new(expand_shorthands()));

  for plugin in normalize_css(&options) {
    pipeline.push(plugin);
  }
  pipeline.push(Box::new(atomicify_rules()));

  if flatten_multiple_selectors_option {
    pipeline.push(Box::new(flatten_multiple_selectors()));
    pipeline.push(Box::new(discard_duplicates()));
  }

  if options.increase_specificity.unwrap_or(false) {
    pipeline.push(Box::new(increase_specificity()));
  }

  let sort_at_rules_option = options.sort_at_rules;
  let sort_shorthand_option = options.sort_shorthand;
  pipeline.push(Box::new(sort_atomic_style_sheet(
    sort_at_rules_option,
    sort_shorthand_option,
  )));

  // Autoprefixer-equivalent vendor prefixing must run after
  // sort-atomic-style-sheet and before whitespace/extract to match Babel.
  // Full Autoprefixer port (wired to browserslist and caniuse data)
  if std::env::var("AUTOPREFIXER")
    .map(|v| v != "off")
    .unwrap_or(true)
  {
    super::plugins::vendor_autoprefixer::AutoprefixerData::init(
      options.browserslist_config_path.as_deref(),
      options.browserslist_env.as_deref(),
    );
    pipeline.push(Box::new(
      super::plugins::vendor_autoprefixer::vendor_autoprefixer(),
    ));
  }
  pipeline.push(Box::new(normalize_whitespace()));
  pipeline.push(Box::new(extract_stylesheets()));

  for plugin in pipeline {
    plugin.run(&mut stylesheet, &mut ctx);
  }

  let serialized_stylesheet = serialize_stylesheet(&stylesheet)?;
  let preserved_comments = ctx.take_preserved_comments();

  let comment_block = preserved_comments.concat();

  if ctx.sheets.is_empty() {
    let mut sheet = serialized_stylesheet;
    if !comment_block.is_empty() {
      sheet = format!("{}{}", comment_block, sheet);
    }
    ctx.push_sheet(sheet);
  } else {
    if !comment_block.is_empty() {
      if let Some(first) = ctx.sheets.first_mut() {
        first.insert_str(0, &comment_block);
      }
    }

    if ctx.sheets.iter().any(|sheet| sheet.is_empty()) {
      for sheet in &mut ctx.sheets {
        if sheet.is_empty() {
          *sheet = serialized_stylesheet.clone();
        }
      }
    }
  }

  Ok(ctx.finish())
}

/// Execute the CSS pipeline.
pub fn transform_css(
  css: &str,
  options: TransformCssOptions,
) -> Result<TransformCssResult, CssTransformError> {
  // Skip empty inputs (only whitespace/semicolons), mirroring Babel which would
  // produce no sheets/class names for an empty declaration block.
  let trimmed = css.trim();
  if trimmed.is_empty() || trimmed.chars().all(|c| c == ';') {
    return Ok(TransformCssResult {
      sheets: Vec::new(),
      class_names: Vec::new(),
    });
  }

  // Handle brace-wrapped declaration blocks (e.g. "{color:red;}") by stripping
  // the braces and reusing the SWC pipeline to mirror Babel's wrap-bare-decls.
  // Allow bare declarations by letting the PostCSS pipeline run with ignore_errors;
  // the wrap-bare-decls plugin will lift them into an empty-selector rule as Babel does.
  // No pre-wrapping here—feed the raw CSS through.

  if std::env::var("STACK_DEBUG").is_ok() {
    eprintln!("[transform_css] postcss path css=\"{}\"", css);
  }

  // Default to the PostCSS engine-backed pipeline when available.
  #[cfg(feature = "postcss_engine")]
  {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[postcss] via-postcss begin");
    }
    let r = transform_css_via_postcss(css, options);
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[postcss] via-postcss end");
    }
    return r;
  }
  #[cfg(not(feature = "postcss_engine"))]
  {
    return transform_css_via_swc_pipeline(css, options);
  }
}

/// Legacy Babel plugin name used in error reporting.
#[allow(dead_code)]
const FALLBACK_PLUGIN_NAME: &str = "@compiled/postcss";

#[cfg(test)]
mod tests {
  use super::*;

  fn transform(css: &str, hash_strategy: Option<HashStrategy>) -> TransformCssResult {
    transform_css_via_swc_pipeline(
      css,
      TransformCssOptions {
        hash_strategy,
        ..Default::default()
      },
    )
    .expect("transform_css failed")
  }

  /// Default strategy: base-36 group hash, 4-char group → 9-char class (_GGGGVVVV).
  /// Group seed: '' + 'undefined' + '&' + 'color' = 'undefined&color' → 'syaz'
  /// Value seed: 'red' → '5scu'
  #[test]
  fn hash_strategy_default_produces_correct_class_name() {
    let result = transform("color: red;", None);
    assert!(
      result.class_names.iter().any(|c| c == "_syaz5scu"),
      "expected _syaz5scu in {:?}",
      result.class_names
    );
  }

  /// Enhanced strategy: base-62 group hash, 4-char group → 9-char class (_GGGGVVVV).
  /// Same class length as default but reduced collision risk.
  /// Group seed: 'undefined&color' → base62 first 4 chars = '1UtD'
  #[test]
  fn hash_strategy_enhanced_produces_correct_class_name() {
    let result = transform("color: red;", Some(HashStrategy::Enhanced));
    assert!(
      result.class_names.iter().any(|c| c == "_1UtD5scu"),
      "expected _1UtD5scu in {:?}",
      result.class_names
    );
  }

  /// Max strategy: base-62 group hash, 6-char group → 11-char class (_GGGGGGVVVV).
  /// Structurally incompatible with default/enhanced — cross-strategy deduplication not supported.
  /// Group seed: 'undefined&color' → base62 first 6 chars = '1UtDYz'
  #[test]
  fn hash_strategy_max_produces_correct_class_name() {
    let result = transform("color: red;", Some(HashStrategy::Max));
    assert!(
      result.class_names.iter().any(|c| c == "_1UtDYz5scu"),
      "expected _1UtDYz5scu in {:?}",
      result.class_names
    );
  }

  /// Default strategy class names are 9 chars (_GGGGVVVV).
  #[test]
  fn hash_strategy_default_class_length_is_9() {
    let result = transform("color: red;", None);
    for class in &result.class_names {
      if class.starts_with('_') && !class.contains(' ') {
        assert_eq!(class.len(), 9, "expected 9-char class, got {:?}", class);
      }
    }
  }

  /// Enhanced strategy class names are 9 chars (_GGGGVVVV).
  #[test]
  fn hash_strategy_enhanced_class_length_is_9() {
    let result = transform("color: red;", Some(HashStrategy::Enhanced));
    for class in &result.class_names {
      if class.starts_with('_') && !class.contains(' ') {
        assert_eq!(class.len(), 9, "expected 9-char class, got {:?}", class);
      }
    }
  }

  /// Max strategy class names are 11 chars (_GGGGGGVVVV).
  #[test]
  fn hash_strategy_max_class_length_is_11() {
    let result = transform("color: red;", Some(HashStrategy::Max));
    for class in &result.class_names {
      if class.starts_with('_') && !class.contains(' ') {
        assert_eq!(class.len(), 11, "expected 11-char class, got {:?}", class);
      }
    }
  }

  /// Different strategies produce different class names for the same input.
  #[test]
  fn hash_strategies_produce_different_class_names() {
    let default_result = transform("color: red;", None);
    let enhanced_result = transform("color: red;", Some(HashStrategy::Enhanced));
    let max_result = transform("color: red;", Some(HashStrategy::Max));

    assert_ne!(
      default_result.class_names, enhanced_result.class_names,
      "default and enhanced should differ"
    );
    assert_ne!(
      default_result.class_names, max_result.class_names,
      "default and max should differ"
    );
    assert_ne!(
      enhanced_result.class_names, max_result.class_names,
      "enhanced and max should differ"
    );
  }
}
