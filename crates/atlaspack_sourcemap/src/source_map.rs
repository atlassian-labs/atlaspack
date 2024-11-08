use std::path::Path;

use parcel_sourcemap::{SourceMap as ParcelSourceMap, SourceMapError};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug)]
pub struct SourceMap {
  inner: ParcelSourceMap,
}

// TODO: Consider forking ParcelSourceMap
impl SourceMap {
  pub fn new(project_root: &Path) -> Self {
    SourceMap {
      inner: ParcelSourceMap::new(&project_root.to_string_lossy()),
    }
  }

  pub fn from_data_url(project_root: &Path, data_url: &str) -> Result<Self, SourceMapError> {
    Ok(SourceMap {
      inner: ParcelSourceMap::from_data_url(&project_root.to_string_lossy(), data_url)?,
    })
  }

  pub fn from_json(project_root: &Path, json: &str) -> Result<SourceMap, SourceMapError> {
    Ok(SourceMap {
      inner: ParcelSourceMap::from_json(&project_root.to_string_lossy(), json)?,
    })
  }

  pub fn extends(&mut self, original_sourcemap: &mut SourceMap) -> Result<(), SourceMapError> {
    self.inner.extends(&mut original_sourcemap.inner)?;

    Ok(())
  }
}

impl From<ParcelSourceMap> for SourceMap {
  fn from(source_map: ParcelSourceMap) -> Self {
    SourceMap { inner: source_map }
  }
}

impl PartialEq for SourceMap {
  fn eq(&self, other: &Self) -> bool {
    // TODO: Add onto this when needed
    self.inner.get_names() == other.inner.get_names()
  }
}

impl Serialize for SourceMap {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let json = self
      .inner
      .clone()
      .to_json(None)
      .map_err(|err| serde::ser::Error::custom(err.to_string()))?;

    serializer.serialize_str(&json)
  }
}

impl<'de> Deserialize<'de> for SourceMap {
  fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    todo!()
  }
}
