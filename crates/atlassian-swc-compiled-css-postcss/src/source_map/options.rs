use std::fmt;
use std::sync::Arc;

use crate::ast::nodes::RootLike;
use serde_json;
use sourcemap::SourceMap;

pub type PrevMapCallback = dyn Fn(Option<&str>) -> Option<String> + Send + Sync;
pub type AnnotationCallback = dyn Fn(Option<&str>, Option<RootLike>) -> String + Send + Sync;

#[derive(Clone)]
pub enum PrevMap {
  Disabled,
  Text(String),
  Function(Arc<PrevMapCallback>),
  SourceMap(Arc<SourceMap>),
  Object(serde_json::Value),
}

impl PrevMap {
  pub fn disabled() -> Self {
    PrevMap::Disabled
  }
}

impl fmt::Debug for PrevMap {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PrevMap::Disabled => f.write_str("PrevMap::Disabled"),
      PrevMap::Text(_) => f.write_str("PrevMap::Text(...)"),
      PrevMap::Function(_) => f.write_str("PrevMap::Function(...)"),
      PrevMap::SourceMap(_) => f.write_str("PrevMap::SourceMap(...)"),
      PrevMap::Object(_) => f.write_str("PrevMap::Object(...)"),
    }
  }
}

#[derive(Clone)]
pub enum MapAnnotation {
  Default,
  Disabled,
  String(String),
  Callback(Arc<AnnotationCallback>),
}

impl Default for MapAnnotation {
  fn default() -> Self {
    MapAnnotation::Default
  }
}

impl fmt::Debug for MapAnnotation {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      MapAnnotation::Default => f.write_str("MapAnnotation::Default"),
      MapAnnotation::Disabled => f.write_str("MapAnnotation::Disabled"),
      MapAnnotation::String(value) => f.debug_tuple("MapAnnotation::String").field(value).finish(),
      MapAnnotation::Callback(_) => f.write_str("MapAnnotation::Callback(...)"),
    }
  }
}

#[derive(Clone, Default)]
pub struct MapOptions {
  pub inline: Option<bool>,
  pub annotation: MapAnnotation,
  pub sources_content: Option<bool>,
  pub prev: Option<PrevMap>,
  pub absolute: bool,
}

#[derive(Clone, Debug)]
pub enum MapSetting {
  Auto,
  Disabled,
  Enabled(MapOptions),
}

impl Default for MapSetting {
  fn default() -> Self {
    MapSetting::Auto
  }
}

impl MapSetting {
  pub fn disabled() -> Self {
    MapSetting::Disabled
  }

  pub fn enabled(opts: MapOptions) -> Self {
    MapSetting::Enabled(opts)
  }

  pub fn auto() -> Self {
    MapSetting::Auto
  }

  pub fn is_explicit(&self) -> bool {
    !matches!(self, MapSetting::Auto)
  }

  pub fn is_enabled(&self) -> bool {
    !matches!(self, MapSetting::Disabled)
  }

  pub fn options(&self) -> MapOptions {
    match self {
      MapSetting::Auto => MapOptions::default(),
      MapSetting::Disabled => MapOptions::default(),
      MapSetting::Enabled(opts) => opts.clone(),
    }
  }

  pub fn options_ref(&self) -> Option<&MapOptions> {
    match self {
      MapSetting::Enabled(opts) => Some(opts),
      _ => None,
    }
  }
}

impl fmt::Debug for MapOptions {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("MapOptions")
      .field("inline", &self.inline)
      .field("annotation", &self.annotation)
      .field("sources_content", &self.sources_content)
      .field("prev", &self.prev)
      .field("absolute", &self.absolute)
      .finish()
  }
}
