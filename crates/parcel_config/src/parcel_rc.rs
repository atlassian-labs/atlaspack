use std::path::PathBuf;

use indexmap::IndexMap;
use atlaspack_core::types::File;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Extends {
  One(String),
  Many(Vec<String>),
}

/// Deserialized .atlaspack_rc config
#[derive(Debug, Deserialize)]
pub struct ParcelRc {
  pub extends: Option<Extends>,
  pub bundler: Option<String>,
  pub compressors: Option<IndexMap<String, Vec<String>>>,
  pub namers: Option<Vec<String>>,
  pub optimizers: Option<IndexMap<String, Vec<String>>>,
  pub packagers: Option<IndexMap<String, String>>,
  pub reporters: Option<Vec<String>>,
  pub resolvers: Option<Vec<String>>,
  pub runtimes: Option<Vec<String>>,
  pub transformers: Option<IndexMap<String, Vec<String>>>,
  pub validators: Option<IndexMap<String, Vec<String>>>,
}

/// Represents the .atlaspack_rc config file
#[derive(Debug)]
pub struct ParcelRcFile {
  pub contents: ParcelRc,
  pub path: PathBuf,
  pub raw: String,
}

impl From<&ParcelRcFile> for File {
  fn from(atlaspack_rc: &ParcelRcFile) -> Self {
    File {
      contents: atlaspack_rc.raw.clone(),
      path: atlaspack_rc.path.clone(),
    }
  }
}
