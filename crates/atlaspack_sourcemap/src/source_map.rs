use std::path::Path;

use parcel_sourcemap::{Mapping, SourceMap as ParcelSourceMap, SourceMapError};
use rkyv::AlignedVec;
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

  pub fn from_buffer(project_root: &Path, buffer: &[u8]) -> Result<Self, SourceMapError> {
    Ok(SourceMap {
      inner: ParcelSourceMap::from_buffer(&project_root.to_string_lossy(), buffer)?,
    })
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

  pub fn get_mappings(&self) -> Vec<Mapping> {
    self.inner.get_mappings()
  }

  pub fn get_names(&self) -> &Vec<String> {
    self.inner.get_names()
  }

  pub fn get_sources(&self) -> &Vec<String> {
    self.inner.get_sources()
  }

  pub fn to_buffer(&self) -> Result<Vec<u8>, SourceMapError> {
    let mut output = AlignedVec::new();

    self.inner.to_buffer(&mut output)?;

    Ok(output.into_vec())
  }

  pub fn to_json(&self) -> Result<String, SourceMapError> {
    self.inner.clone().to_json(None)
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
      .to_json()
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

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use parcel_sourcemap::OriginalLocation;
  use pretty_assertions::assert_eq;

  use super::*;

  // Note that vlq mappings are 0 based
  // When mappings are read in JavaScript, they will go through [`mapping_to_js_object`](https://github.com/parcel-bundler/source-map/blob/master/parcel_sourcemap_node/src/lib.rs)
  // and become 1 based when serializing.

  #[test]
  fn buffer_contains_correct_data() -> anyhow::Result<()> {
    let (json, expected) = json_fixture();

    let source_map = SourceMap::from_json(&PathBuf::default(), &json)?;
    let buffer = source_map.to_buffer()?;
    let source_map = SourceMap::from_buffer(&PathBuf::default(), &buffer)?;

    assert_eq!(source_map.get_names(), &expected.names);
    assert_eq!(source_map.get_sources(), &expected.sources);
    assert_eq!(source_map.inner.get_mappings(), expected.mappings);

    Ok(())
  }

  #[test]
  fn buffer_conversion_is_lossless() -> anyhow::Result<()> {
    let (json, _) = json_fixture();

    let source_map = SourceMap::from_json(&PathBuf::default(), &json)?;
    let buffer = source_map.to_buffer()?;

    let source_map = SourceMap::from_buffer(&PathBuf::default(), &buffer)?;
    let expected_buffer = source_map.to_buffer()?;

    assert_eq!(buffer, expected_buffer);

    Ok(())
  }

  #[test]
  fn json_contains_correct_data() -> anyhow::Result<()> {
    let (json, expected) = json_fixture();

    let source_map = SourceMap::from_json(&PathBuf::default(), &json)?;

    assert_eq!(source_map.get_names(), &expected.names);
    assert_eq!(source_map.get_sources(), &expected.sources);
    assert_eq!(source_map.get_mappings(), expected.mappings);

    Ok(())
  }

  #[test]
  fn json_conversion_is_lossless() -> anyhow::Result<()> {
    let (json, expected) = json_fixture();

    let source_map = SourceMap::from_json(&PathBuf::default(), &json)?;

    assert_eq!(source_map.to_json()?, expected.json);

    Ok(())
  }

  struct ExpectedSourceMap {
    json: String,
    mappings: Vec<Mapping>,
    names: Vec<String>,
    sources: Vec<String>,
  }

  fn json_fixture() -> (String, ExpectedSourceMap) {
    let mut expected_json = r#"
      {
        "version": 3,
        "sourceRoot": null,
        "mappings": "AAAA,SAAS;IACP,OAAO;AACT;AAEA,iBAAiB",
        "sources": ["index.js"],
        "sourcesContent": [""],
        "names":[]
      }"#
      .to_string();

    expected_json.retain(|c| !c.is_whitespace());

    (
      String::from(
        r#"
          {
            "mappings": "AAAA,SAAS;IACP,OAAO;AACT;AAEA,iBAAiB",
            "names": [],
            "sources": ["index.js"]
          }
        "#,
      ),
      ExpectedSourceMap {
        json: expected_json,
        mappings: vec![
          Mapping {
            generated_line: 0,
            generated_column: 0,
            original: Some(OriginalLocation {
              original_line: 0,
              original_column: 0,
              source: 0,
              name: None,
            }),
          },
          Mapping {
            generated_line: 0,
            generated_column: 9,
            original: Some(OriginalLocation {
              original_line: 0,
              original_column: 9,
              source: 0,
              name: None,
            }),
          },
          Mapping {
            generated_line: 1,
            generated_column: 4,
            original: Some(OriginalLocation {
              original_line: 1,
              original_column: 2,
              source: 0,
              name: None,
            }),
          },
          Mapping {
            generated_line: 1,
            generated_column: 11,
            original: Some(OriginalLocation {
              original_line: 1,
              original_column: 9,
              source: 0,
              name: None,
            }),
          },
          Mapping {
            generated_line: 2,
            generated_column: 0,
            original: Some(OriginalLocation {
              original_line: 2,
              original_column: 0,
              source: 0,
              name: None,
            }),
          },
          Mapping {
            generated_line: 3,
            generated_column: 0,
            original: Some(OriginalLocation {
              original_line: 4,
              original_column: 0,
              source: 0,
              name: None,
            }),
          },
          Mapping {
            generated_line: 3,
            generated_column: 17,
            original: Some(OriginalLocation {
              original_line: 4,
              original_column: 17,
              source: 0,
              name: None,
            }),
          },
        ],
        names: Vec::new(),
        sources: vec![String::from("index.js")],
      },
    )
  }
}
