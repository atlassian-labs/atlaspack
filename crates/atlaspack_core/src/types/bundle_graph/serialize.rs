use std::collections::{HashMap, HashSet};

use serde::Deserializer;

use crate::types::{Dependency, UsedSymbolResolution};

/// Custom deserializer for JavaScript Set - handles both array and object/map representations
/// JavaScript Sets are often serialized as objects/maps, but we need them as arrays/sequences
#[tracing::instrument(level = "trace", skip_all)]
pub fn deserialize_set_from_array_or_map<'de, D>(
  deserializer: D,
) -> Result<HashSet<String>, D::Error>
where
  D: Deserializer<'de>,
{
  use serde::de::{self, Visitor};
  use std::fmt;

  struct SetVisitor;

  impl<'de> Visitor<'de> for SetVisitor {
    type Value = HashSet<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("an array or object representing a Set")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
      A: de::SeqAccess<'de>,
    {
      let mut set = HashSet::new();
      while let Some(item) = seq.next_element::<String>()? {
        set.insert(item);
      }
      Ok(set)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
      A: de::MapAccess<'de>,
    {
      // JavaScript Set serialized as object: extract keys
      let mut set = HashSet::new();
      while let Some((key, _)) = map.next_entry::<String, serde::de::IgnoredAny>()? {
        set.insert(key);
      }
      Ok(set)
    }
  }

  deserializer.deserialize_any(SetVisitor)
}

/// Custom deserializer for JavaScript Map - handles object representation for UsedSymbolResolution
#[tracing::instrument(level = "trace", skip_all)]
pub fn deserialize_map_from_js_map<'de, D>(
  deserializer: D,
) -> Result<HashMap<String, Option<UsedSymbolResolution>>, D::Error>
where
  D: Deserializer<'de>,
{
  use serde::de::{self, Visitor};
  use std::fmt;

  struct MapVisitor;

  impl<'de> Visitor<'de> for MapVisitor {
    type Value = HashMap<String, Option<UsedSymbolResolution>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("a map/object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
      A: de::MapAccess<'de>,
    {
      let mut result = HashMap::new();
      while let Some((key, value)) = map.next_entry::<String, Option<UsedSymbolResolution>>()? {
        result.insert(key, value);
      }
      Ok(result)
    }
  }

  deserializer.deserialize_map(MapVisitor)
}

/// Custom deserializer for JavaScript Map containing Dependencies - handles object representation
#[tracing::instrument(level = "trace", skip_all)]
pub fn deserialize_dependency_map_from_js_map<'de, D>(
  deserializer: D,
) -> Result<HashMap<String, Dependency>, D::Error>
where
  D: Deserializer<'de>,
{
  use serde::de::{self, Visitor};
  use std::fmt;

  struct DependencyMapVisitor;

  impl<'de> Visitor<'de> for DependencyMapVisitor {
    type Value = HashMap<String, Dependency>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("a map/object of dependencies")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
      A: de::MapAccess<'de>,
    {
      let mut result = HashMap::new();
      while let Some((key, value)) = map.next_entry::<String, Dependency>()? {
        result.insert(key, value);
      }
      Ok(result)
    }
  }

  deserializer.deserialize_map(DependencyMapVisitor)
}
