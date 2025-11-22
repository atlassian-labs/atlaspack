/// Partial configuration for CompiledCssInJs transform.
/// All fields are optional and will use defaults if not specified.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CompiledCssInJsConfig {
  ///
  /// Path to the compiled config file. Specified by the JS transformer
  ///
  pub config_path: Option<String>,
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
  /// Defaults to `["@compiled/react"]`
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
  /// Whether to sort shorthand properties.
  /// Defaults to `true`
  pub sort_shorthand: Option<bool>,
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
  ///
  /// Build a map of safe assets for migration purposes.
  ///
  /// Defaults to `false`.
  ///
  pub unsafe_report_safe_assets_for_migration: Option<bool>,
  ///
  /// Use known safe assets from hash for migration purposes.
  ///
  /// Requires `unsafe_report_safe_assets_for_migration` to build a map of safe assets.
  ///
  /// Defaults to `false`.
  ///
  pub unsafe_use_safe_assets: Option<bool>,
}

/// Full configuration for CompiledCssInJs transform.
/// All fields are required.
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledCssInJsTransformConfig {
  ///
  /// Path to the compiled config file. Specified by the JS transformer
  ///
  pub config_path: Option<String>,
  ///
  /// Will import the React namespace if it is missing.
  /// When using the `'automatic'` jsx runtime set this to `false`.
  ///
  /// Defaults to `true`
  ///
  pub import_react: bool,
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
  pub import_sources: Vec<String>,
  ///
  /// Will run additional cssnano plugins to normalize CSS during build.
  ///
  /// Defaults to `true`
  ///
  pub optimize_css: bool,
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
  pub add_component_name: bool,
  ///
  /// Whether Compiled should process usages of xcss in the codebase.
  /// Disable this if xcss is not implemented in your codebase using Compiled's xcss functionality.
  ///
  /// Defaults to `true`
  ///
  pub process_xcss: bool,
  ///
  /// Increases the specificity of all declared Compiled styles.
  /// Generally you would only use this for migration purposes when mixing two or more styling
  /// solutions.
  ///
  /// Defaults to `false`
  pub increase_specificity: bool,
  ///
  /// Whether to sort at-rules, including media queries.
  /// Defaults to `true`
  pub sort_at_rules: bool,
  ///
  /// Whether to sort shorthand properties.
  /// Defaults to `true`
  pub sort_shorthand: bool,
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
  pub flatten_multiple_selectors: bool,
  ///
  /// Extract CSS from from the code into a single stylesheet
  ///
  /// Defaults to `false`.
  pub extract: bool,
  ///
  /// Builds in a node environment.
  ///
  /// Defaults to `false`.
  pub ssr: bool,
  ///
  /// Build a map of safe assets for migration purposes.
  ///
  /// Defaults to `false`.s
  ///
  pub unsafe_report_safe_assets_for_migration: bool,
  ///
  /// Use known safe assets from hash for migration purposes.
  ///
  /// Requires `unsafe_report_safe_assets_for_migration` to build a map of safe assets.
  ///
  /// Defaults to `false`.
  ///
  pub unsafe_use_safe_assets: bool,
}

impl Default for CompiledCssInJsTransformConfig {
  fn default() -> Self {
    Self {
      config_path: None,
      add_component_name: false,
      class_hash_prefix: None,
      extensions: None,
      extract: false,
      flatten_multiple_selectors: true,
      import_react: true,
      import_sources: vec!["@compiled/react".into()],
      increase_specificity: false,
      nonce: None,
      optimize_css: true,
      process_xcss: true,
      sort_at_rules: true,
      sort_shorthand: true,
      ssr: false,
      unsafe_report_safe_assets_for_migration: false,
      unsafe_use_safe_assets: false,
    }
  }
}

impl From<CompiledCssInJsConfig> for CompiledCssInJsTransformConfig {
  fn from(partial: CompiledCssInJsConfig) -> Self {
    let defaults = Self::default();
    Self {
      config_path: partial.config_path,
      import_react: partial.import_react.unwrap_or(defaults.import_react),
      nonce: partial.nonce.or(defaults.nonce),
      import_sources: partial.import_sources.unwrap_or(defaults.import_sources),
      optimize_css: partial.optimize_css.unwrap_or(defaults.optimize_css),
      extensions: partial.extensions.or(defaults.extensions),
      add_component_name: partial
        .add_component_name
        .unwrap_or(defaults.add_component_name),
      process_xcss: partial.process_xcss.unwrap_or(defaults.process_xcss),
      increase_specificity: partial
        .increase_specificity
        .unwrap_or(defaults.increase_specificity),
      sort_at_rules: partial.sort_at_rules.unwrap_or(defaults.sort_at_rules),
      sort_shorthand: partial.sort_shorthand.unwrap_or(defaults.sort_shorthand),
      class_hash_prefix: partial.class_hash_prefix.or(defaults.class_hash_prefix),
      flatten_multiple_selectors: partial
        .flatten_multiple_selectors
        .unwrap_or(defaults.flatten_multiple_selectors),
      extract: partial.extract.unwrap_or(defaults.extract),
      ssr: partial.ssr.unwrap_or(defaults.ssr),
      unsafe_report_safe_assets_for_migration: partial
        .unsafe_report_safe_assets_for_migration
        .unwrap_or(defaults.unsafe_report_safe_assets_for_migration),
      unsafe_use_safe_assets: partial
        .unsafe_use_safe_assets
        .unwrap_or(defaults.unsafe_use_safe_assets),
    }
  }
}
