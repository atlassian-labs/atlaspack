use atlaspack_js_swc_core::JsxOptions;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum InlineEnvironment {
  Enabled(bool),
  Environments(Vec<String>),
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

  pub jsx: Option<JsxOptions>,
}

#[derive(Deserialize)]
pub struct JsTransformerPackageJson {
  #[serde(rename = "@atlaspack/transformer-js")]
  pub config: Option<JsTransformerConfig>,
}
