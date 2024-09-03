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
  pub inline_fs: Option<bool>,
}
