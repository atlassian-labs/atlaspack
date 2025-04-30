use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PackageJson {
  pub name: Option<String>,
  pub version: Option<String>,
  pub atlaspack: Option<PackageJsonAtlaspack>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PackageJsonAtlaspack {
  pub version: Option<String>,
  pub versions: Option<HashMap<String, String>>,
}

impl PackageJson {
  pub fn parse<S: AsRef<str>>(input: S) -> anyhow::Result<Self> {
    Ok(serde_json::from_str(input.as_ref())?)
  }

  pub fn parse_from_file<P: AsRef<Path>>(input: P) -> anyhow::Result<Self> {
    let input = fs::read_to_string(input.as_ref())?;
    Self::parse(input)
  }
}
