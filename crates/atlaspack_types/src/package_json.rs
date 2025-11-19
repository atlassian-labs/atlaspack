use bitflags::bitflags;
use serde::Deserialize;

bitflags! {
  /// Represents a condition name within the exports field of package.json
  #[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
  pub struct ExportsCondition: u16 {
    const IMPORT = 1 << 0;
    const REQUIRE = 1 << 1;
    const MODULE = 1 << 2;
    const NODE = 1 << 3;
    const BROWSER = 1 << 4;
    const WORKER = 1 << 5;
    const WORKLET = 1 << 6;
    const ELECTRON = 1 << 7;
    const DEVELOPMENT = 1 << 8;
    const PRODUCTION = 1 << 9;
    const TYPES = 1 << 10;
    const DEFAULT = 1 << 11;
    const STYLE = 1 << 12;
    const SASS = 1 << 13;
  }
}

impl Default for ExportsCondition {
  fn default() -> Self {
    ExportsCondition::empty()
  }
}

impl serde::Serialize for ExportsCondition {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    self.bits().serialize(serializer)
  }
}

impl<'de> serde::Deserialize<'de> for ExportsCondition {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum JSExportsCondition {
      StringArray(Vec<String>),
      Int(u16),
    }

    let condition: JSExportsCondition = Deserialize::deserialize(deserializer)?;
    match condition {
      JSExportsCondition::Int(bits) => Ok(ExportsCondition::from_bits_truncate(bits)),
      JSExportsCondition::StringArray(array) => {
        let mut bits = ExportsCondition::empty();
        for item in array {
          let condition = ExportsCondition::try_from(item.as_str())
            .map_err(|_| serde::de::Error::custom(format!("Invalid export condition {item}")))?;
          bits |= condition;
        }
        Ok(bits)
      }
    }
  }
}

impl TryFrom<&str> for ExportsCondition {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Ok(match value {
      "import" => ExportsCondition::IMPORT,
      "require" => ExportsCondition::REQUIRE,
      "module" => ExportsCondition::MODULE,
      "node" => ExportsCondition::NODE,
      "browser" => ExportsCondition::BROWSER,
      "worker" => ExportsCondition::WORKER,
      "worklet" => ExportsCondition::WORKLET,
      "electron" => ExportsCondition::ELECTRON,
      "development" => ExportsCondition::DEVELOPMENT,
      "production" => ExportsCondition::PRODUCTION,
      "types" => ExportsCondition::TYPES,
      "default" => ExportsCondition::DEFAULT,
      "style" => ExportsCondition::STYLE,
      "sass" => ExportsCondition::SASS,
      _ => return Err(()),
    })
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_deserialize_from_bits() {
    let condition: ExportsCondition = serde_json::from_str("1").unwrap();
    assert_eq!(condition, ExportsCondition::IMPORT);
  }

  #[test]
  fn test_deserialize_from_strings() {
    let condition: ExportsCondition = serde_json::from_str("[\"import\", \"sass\"]").unwrap();
    assert_eq!(condition, ExportsCondition::IMPORT | ExportsCondition::SASS);
  }
}
