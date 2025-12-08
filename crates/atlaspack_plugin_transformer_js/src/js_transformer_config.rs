use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum InlineEnvironment {
  Enabled(bool),
  Environments(Vec<String>),
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum AutomaticReactRuntime {
  Enabled(bool),
  Glob(Vec<String>),
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactOptions {
  pub automatic_runtime: Option<AutomaticReactRuntime>,
  pub jsx_pragma: Option<String>,
  pub jsx_pragma_fragment: Option<String>,
  pub jsx_import_source: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsTransformerConfig {
  #[serde(rename = "unstable_inlineConstants")]
  pub inline_constants: Option<bool>,
  pub inline_environment: Option<InlineEnvironment>,

  #[serde(rename = "inlineFS")]
  pub inline_fs: Option<bool>,

  pub magic_comments: Option<bool>,

  pub add_react_display_name: Option<bool>,

  pub react: Option<ReactOptions>,
}

#[derive(Deserialize)]
pub struct JsTransformerPackageJson {
  #[serde(rename = "@atlaspack/transformer-js")]
  pub config: Option<JsTransformerConfig>,
}
