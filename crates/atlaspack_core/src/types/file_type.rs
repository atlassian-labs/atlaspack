use std::hash::Hash;

use serde::Deserialize;
use serde::Serialize;

/// Represents a file type by its extension
///
/// Defaults to `FileType::Js` for convenience.
#[derive(Default, Debug, Clone, PartialEq, Hash)]
pub enum FileType {
  Avif,
  Css,
  Gif,
  Html,
  #[default]
  Js,
  Json,
  Jpeg,
  Png,
  Jsx,
  Tiff,
  Ts,
  Tsx,
  WebP,
  Other(String),
}

impl Serialize for FileType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    self.extension().serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for FileType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let ext: String = Deserialize::deserialize(deserializer)?;
    Ok(Self::from_extension(&ext))
  }
}

impl FileType {
  pub fn extension(&self) -> &str {
    match self {
      FileType::Js => "js",
      FileType::Jsx => "jsx",
      FileType::Ts => "ts",
      FileType::Tsx => "tsx",
      FileType::Css => "css",
      FileType::Json => "json",
      FileType::Jpeg => "jpeg",
      FileType::Png => "png",
      FileType::Gif => "gif",
      FileType::Html => "html",
      FileType::Avif => "avif",
      FileType::Tiff => "tiff",
      FileType::WebP => "webp",
      FileType::Other(s) => s.as_str(),
    }
  }

  pub fn from_extension(ext: &str) -> Self {
    match ext {
      "js" => FileType::Js,
      "mjs" => FileType::Js,
      "cjs" => FileType::Js,
      "jsx" => FileType::Jsx,
      "ts" => FileType::Ts,
      "tsx" => FileType::Tsx,
      "css" => FileType::Css,
      "json" => FileType::Json,
      "jpg" => FileType::Jpeg,
      "jpeg" => FileType::Jpeg,
      "png" => FileType::Png,
      "gif" => FileType::Gif,
      "html" => FileType::Html,
      "avif" => FileType::Avif,
      "avifs" => FileType::Avif,
      "tiff" => FileType::Tiff,
      "webp" => FileType::WebP,
      ext => FileType::Other(ext.to_string()),
    }
  }
}
