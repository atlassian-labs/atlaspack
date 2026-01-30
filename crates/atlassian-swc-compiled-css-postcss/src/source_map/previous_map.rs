use std::cell::RefCell;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use percent_encoding::percent_decode_str;
use serde_json;
use serde_json::{Map as JsonMap, Value};
use sourcemap::SourceMap;

use super::options::{MapOptions, MapSetting, PrevMap};

#[derive(Debug)]
pub struct PreviousMapError(pub String);

impl std::fmt::Display for PreviousMapError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for PreviousMapError {}

impl From<String> for PreviousMapError {
  fn from(value: String) -> Self {
    PreviousMapError(value)
  }
}

#[derive(Clone, Debug)]
pub struct PreviousMap {
  pub text: String,
  pub file: Option<PathBuf>,
  pub root: Option<PathBuf>,
  pub annotation: Option<String>,
  inline: bool,
  consumer: RefCell<Option<Arc<SourceMap>>>,
}

impl PreviousMap {
  pub fn new(
    css: &str,
    from: Option<&str>,
    setting: &MapSetting,
  ) -> Result<Option<Self>, PreviousMapError> {
    if let MapSetting::Disabled = setting {
      return Ok(None);
    }

    let opts = setting.options_ref().cloned().unwrap_or_default();
    let annotation = load_annotation(css);
    let inline = annotation
      .as_ref()
      .map(|ann| ann.trim_start().starts_with("data:"))
      .unwrap_or(false);

    let (text, map_path) = match load_map(from, &opts, inline, annotation.as_deref())? {
      Some(pair) => pair,
      None => return Ok(None),
    };

    let mut file = map_path;

    if file.is_none() {
      if let Some(from_path) = from {
        file = Some(PathBuf::from(from_path));
      }
    }

    let root = file
      .as_ref()
      .and_then(|path| path.parent().map(|parent| parent.to_path_buf()));

    Ok(Some(Self {
      text,
      file,
      root,
      annotation,
      inline,
      consumer: RefCell::new(None),
    }))
  }

  pub fn consumer(&self) -> Arc<SourceMap> {
    if let Some(existing) = self.consumer.borrow().clone() {
      return existing;
    }
    let map = SourceMap::from_slice(self.text.as_bytes())
      .unwrap_or_else(|err| panic!("Failed to parse previous source map: {}", err));
    let arc = Arc::new(map);
    *self.consumer.borrow_mut() = Some(arc.clone());
    arc
  }

  pub fn inline(&self) -> bool {
    self.inline
  }

  pub fn with_content(&self) -> bool {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.text) {
      if let Some(contents) = value.get("sourcesContent").and_then(|v| v.as_array()) {
        return contents.iter().any(|item| !item.is_null());
      }
    }
    false
  }

  /// Recreate a [`PreviousMap`] instance from the serialized object emitted by
  /// PostCSS' `Input#toJSON` helper.
  pub fn from_json_value(value: &Value) -> Result<Self, PreviousMapError> {
    let object = value
      .as_object()
      .ok_or_else(|| PreviousMapError("Previous map JSON must be an object".into()))?;

    let text = if let Some(text) = object.get("text").and_then(|v| v.as_str()) {
      text.to_string()
    } else {
      serde_json::to_string(value).map_err(|_| {
        PreviousMapError("Failed to serialize previous map object into JSON text".to_string())
      })?
    };

    let file = object
      .get("file")
      .or_else(|| object.get("mapFile"))
      .and_then(|v| v.as_str())
      .map(PathBuf::from);

    let root = object
      .get("root")
      .and_then(|v| v.as_str())
      .map(PathBuf::from);

    let annotation = object
      .get("annotation")
      .and_then(|v| v.as_str())
      .map(|s| s.to_string());

    let inline = object
      .get("inline")
      .and_then(|v| v.as_bool())
      .unwrap_or(false);

    Ok(Self {
      text,
      file,
      root,
      annotation,
      inline,
      consumer: RefCell::new(None),
    })
  }

  pub fn to_json_value(&self) -> Value {
    let mut object = JsonMap::new();
    object.insert("text".to_string(), Value::String(self.text.clone()));
    if let Some(file) = &self.file {
      object.insert(
        "file".to_string(),
        Value::String(file.to_string_lossy().to_string()),
      );
    }
    if let Some(root) = &self.root {
      object.insert(
        "root".to_string(),
        Value::String(root.to_string_lossy().to_string()),
      );
    }
    if let Some(annotation) = &self.annotation {
      object.insert("annotation".to_string(), Value::String(annotation.clone()));
    }
    if self.inline {
      object.insert("inline".to_string(), Value::Bool(true));
    }
    Value::Object(object)
  }
}

fn load_annotation(css: &str) -> Option<String> {
  let mut search_start = css.len();
  while let Some(idx) = css[..search_start].rfind("/*") {
    let comment = &css[idx + 2..];
    if let Some(end_idx) = comment.find("*/") {
      let body = &comment[..end_idx];
      let trimmed = body.trim();
      if trimmed.starts_with('#') {
        let rest = trimmed.trim_start_matches('#').trim_start();
        if rest.starts_with("sourceMappingURL=") {
          let url = rest.trim_start_matches("sourceMappingURL=").trim();
          return Some(url.to_string());
        }
      }
      search_start = idx;
    } else {
      break;
    }
  }
  None
}

fn read_file(path: &Path) -> Result<String, PreviousMapError> {
  let mut file = fs::File::open(path).map_err(|_| {
    PreviousMapError(format!(
      "Unable to load previous source map: {}",
      path.display()
    ))
  })?;
  let mut buf = String::new();
  file.read_to_string(&mut buf).map_err(|_| {
    PreviousMapError(format!(
      "Unable to read previous source map: {}",
      path.display()
    ))
  })?;
  Ok(buf.trim().to_string())
}

fn decode_inline(text: &str) -> Result<String, PreviousMapError> {
  let trimmed = text.trim();
  if let Some((prefix, data)) = trimmed.split_once(',') {
    let prefix_lower = prefix.to_ascii_lowercase();
    if prefix_lower.contains("base64") {
      let decoded = BASE64
        .decode(data.as_bytes())
        .map_err(|_| PreviousMapError("Failed to decode base64-encoded source map".to_string()))?;
      return String::from_utf8(decoded)
        .map_err(|_| PreviousMapError("Decoded source map is not valid UTF-8".to_string()));
    } else {
      let decoded = percent_decode_str(data)
        .decode_utf8()
        .map_err(|_| PreviousMapError("Failed to decode percent-encoded source map".to_string()))?;
      return Ok(decoded.into_owned());
    }
  }
  Err(PreviousMapError(
    "Unsupported source map encoding".to_string(),
  ))
}

fn load_map(
  from: Option<&str>,
  opts: &MapOptions,
  inline: bool,
  annotation: Option<&str>,
) -> Result<Option<(String, Option<PathBuf>)>, PreviousMapError> {
  if let Some(prev) = &opts.prev {
    return match prev {
      PrevMap::Disabled => Ok(None),
      PrevMap::Text(text) => Ok(Some((text.clone(), None))),
      PrevMap::Function(callback) => {
        if let Some(path) = callback(from) {
          let map_path = PathBuf::from(path);
          let text = read_file(&map_path)?;
          Ok(Some((text, Some(map_path))))
        } else {
          Ok(None)
        }
      }
      PrevMap::SourceMap(source_map) => {
        let mut buf = Vec::new();
        source_map
          .to_writer(&mut buf)
          .map_err(|_| PreviousMapError("Failed to serialize previous source map".to_string()))?;
        let text = String::from_utf8(buf)
          .map_err(|_| PreviousMapError("Serialized source map is not UTF-8".to_string()))?;
        Ok(Some((text, None)))
      }
      PrevMap::Object(value) => {
        let text = serde_json::to_string(value).map_err(|_| {
          PreviousMapError("Failed to serialize previous source map object".to_string())
        })?;
        Ok(Some((text, None)))
      }
    };
  }

  if inline {
    if let Some(annotation) = annotation {
      let text = decode_inline(annotation)?;
      return Ok(Some((text, None)));
    }
  }

  if let Some(annotation) = annotation {
    let map_path = resolve_map_path(from, annotation);
    let text = read_file(&map_path)?;
    return Ok(Some((text, Some(map_path))));
  }

  Ok(None)
}

fn resolve_map_path(from: Option<&str>, annotation: &str) -> PathBuf {
  if annotation.contains("://") {
    return PathBuf::from(annotation);
  }
  if let Some(from_path) = from {
    let from = Path::new(from_path);
    if let Some(dir) = from.parent() {
      return dir.join(annotation);
    }
  }
  PathBuf::from(annotation)
}
