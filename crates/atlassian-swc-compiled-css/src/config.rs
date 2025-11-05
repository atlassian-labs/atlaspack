#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Clone, Debug)]
pub struct CompiledCssInJsTransformConfig {
  ///
  /// Will import the React namespace if it is missing.
  /// When using the `'automatic'` jsx runtime set this to `false`.
  ///
  /// Defaults to `true`
  ///
  pub import_react: Option<bool>,
  ///
  /// Security nonce that will be applied to inline style elements if defined.
  ///
  /// Defaults to `None`
  ///
  pub nonce: Option<String>,
  ///
  /// Custom module origins that Compiled should compile when using APIs from.
  ///
  /// Defaults to `None`
  ///
  pub import_sources: Option<Vec<String>>,
  ///
  /// Will run additional cssnano plugins to normalize CSS during build.
  ///
  /// Defaults to `true`
  ///
  pub optimize_css: Option<bool>,
  ///
  /// List of file extensions to traverse as code
  ///
  /// Defaults to `None`
  ///
  pub extensions: Option<Vec<String>>,
  ///
  /// Add the component name as class name to DOM in non-production environment if styled is used
  //
  /// Defaults to `false`
  ///
  pub add_component_name: Option<bool>,
  ///
  /// Whether Compiled should process usages of xcss in the codebase.
  /// Disable this if xcss is not implemented in your codebase using Compiled's xcss functionality.
  ///
  /// Defaults to `true`
  ///
  pub process_xcss: Option<bool>,
  ///
  /// Increases the specificity of all declared Compiled styles.
  /// Generally you would only use this for migration purposes when mixing two or more styling
  /// solutions.
  ///
  /// Defaults to `false`
  pub increase_specificity: Option<bool>,
  ///
  /// Whether to sort at-rules, including media queries.
  /// Defaults to `true`
  pub sort_at_rules: Option<bool>,
  ///
  /// Adds a defined prefix to the generated classes' hashes.
  /// Useful in micro frontend environments to avoid clashing/specificity issues.
  ///
  /// Avoid mixing this with extraction as this may throw an error if combined with extraction
  /// or `extract: true` in Webpack loaders or Parcel tranformers.
  ///
  /// Defaults to `None`
  ///
  pub class_hash_prefix: Option<String>,
  ///
  /// Whether to flatten multiple selectors into separate rules to better deduplicate and sort styles.
  ///
  /// Defaults to `true`.
  pub flatten_multiple_selectors: Option<bool>,
  ///
  /// Extract CSS from from the code into a single stylesheet
  ///
  /// Defaults to `false`.
  pub extract: Option<bool>,
  ///
  /// Builds in a node environment.
  ///
  /// Defaults to `false`.
  pub ssr: Option<bool>,
}

impl Default for CompiledCssInJsTransformConfig {
  fn default() -> Self {
    Self {
      add_component_name: None,
      class_hash_prefix: None,
      extensions: None,
      extract: None,
      flatten_multiple_selectors: Some(true),
      import_react: Some(true),
      import_sources: None,
      increase_specificity: None,
      nonce: None,
      optimize_css: Some(true),
      process_xcss: Some(true),
      sort_at_rules: Some(true),
      ssr: None,
    }
  }
}
