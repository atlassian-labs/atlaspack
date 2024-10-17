use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssModulesFullConfig {
  pub global: Option<bool>,
  pub dashed_idents: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CssModulesConfig {
  GlobalOnly(bool),
  Full(CssModulesFullConfig),
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssTransformerConfig {
  pub css_modules: Option<CssModulesConfig>,
}
