use anyhow::anyhow;
use atlaspack_core::types::Diagnostic;
use serde::Serialize;

pub enum AtlaspackError {
  Diagnostic(Diagnostic),
  String(String),
}

impl Serialize for AtlaspackError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match self {
      AtlaspackError::Diagnostic(diagnostic) => diagnostic.serialize(serializer),
      AtlaspackError::String(message) => message.serialize(serializer),
    }
  }
}

impl From<&anyhow::Error> for AtlaspackError {
  fn from(error: &anyhow::Error) -> Self {
    if let Some(diagnostic) = error.downcast_ref::<Diagnostic>() {
      Self::Diagnostic(diagnostic.clone())
    } else if let Some(message) = error.downcast_ref::<String>() {
      Self::String(message.clone())
    } else {
      Self::String(error.to_string())
    }
  }
}

impl From<AtlaspackError> for anyhow::Error {
  fn from(value: AtlaspackError) -> Self {
    match value {
      AtlaspackError::Diagnostic(diagnostic) => anyhow!(diagnostic),
      AtlaspackError::String(message) => anyhow!(message),
    }
  }
}
