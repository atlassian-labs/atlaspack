use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokensTransformerConfig {
  pub token_data_path: String,
  pub should_use_auto_fallback: Option<bool>,
  pub should_force_auto_fallback: Option<bool>,
  pub force_auto_fallback_exemptions: Option<Vec<String>>,
  pub default_theme: Option<String>,
}

#[derive(Deserialize)]
pub struct PackageJson {
  #[serde(rename = "@atlaspack/transformer-tokens")]
  pub config: Option<TokensTransformerConfig>,
}
