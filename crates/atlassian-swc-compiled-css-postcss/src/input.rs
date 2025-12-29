use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::css_syntax_error::CssSyntaxError;
use crate::source_map::{MapSetting, PreviousMap, PreviousMapError};
use serde_json::{Map as JsonMap, Value as JsonValue};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Position {
  pub line: u32,
  pub column: u32,
  pub offset: usize,
}

impl Position {
  pub fn new(line: u32, column: u32, offset: usize) -> Self {
    Self {
      line,
      column,
      offset,
    }
  }
}

#[derive(Clone, Debug, Default)]
pub struct InputOptions {
  pub from: Option<String>,
  pub map: MapSetting,
}

#[derive(Clone, Debug)]
pub struct Input {
  css: String,
  pub from: Option<String>,
  pub file: Option<String>,
  pub id: Option<String>,
  pub has_bom: bool,
  pub map: Option<PreviousMap>,
}

impl Input {
  pub fn new(css: impl Into<String>, opts: InputOptions) -> Result<Self, PreviousMapError> {
    let mut css = css.into();
    let has_bom = css.starts_with('\u{FEFF}') || css.starts_with('\u{FFFE}');
    if has_bom {
      css = css.chars().skip(1).collect();
    }

    let from = opts.from.clone();
    let file = from.clone();
    let id = if file.is_none() {
      static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
      let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
      Some(format!("<input css {:06}>", unique))
    } else {
      None
    };

    let mut map = PreviousMap::new(&css, from.as_deref(), &opts.map)?;
    if let Some(previous) = map.as_mut() {
      if previous.file.is_none() {
        if let Some(from_path) = &from {
          let file_path = PathBuf::from(from_path);
          previous.root = file_path.parent().map(|parent| parent.to_path_buf());
          previous.file = Some(file_path);
        }
      }
    }

    Ok(Self {
      css,
      from,
      file,
      id,
      has_bom,
      map,
    })
  }

  pub fn css(&self) -> &str {
    &self.css
  }

  pub fn error(
    &self,
    message: impl Into<String>,
    start: Position,
    end: Option<Position>,
  ) -> CssSyntaxError {
    self.error_with_plugin(message, start, end, None)
  }

  pub fn error_with_plugin(
    &self,
    message: impl Into<String>,
    start: Position,
    end: Option<Position>,
    plugin: Option<String>,
  ) -> CssSyntaxError {
    CssSyntaxError::from_input(
      message,
      Some(InputRef::new(self.clone())),
      start,
      end,
      plugin,
    )
  }

  pub fn from_offset(&self, offset: usize) -> Position {
    let mut line = 1;
    let mut column = 1;
    let mut current_offset = 0;
    for ch in self.css.chars() {
      if current_offset == offset {
        return Position {
          line,
          column,
          offset,
        };
      }
      current_offset += ch.len_utf8();
      if ch == '\n' {
        line += 1;
        column = 1;
      } else {
        column += 1;
      }
    }
    Position {
      line,
      column,
      offset,
    }
  }

  /// Rehydrate an [`Input`] instance from serialized JSON fields as produced by
  /// `Node#toJSON` in the canonical PostCSS distribution.
  pub fn from_hydrated(
    css: String,
    file: Option<String>,
    id: Option<String>,
    has_bom: bool,
    map: Option<PreviousMap>,
  ) -> Self {
    let from = file.clone().or_else(|| id.clone());
    Self {
      css,
      from,
      file,
      id,
      has_bom,
      map,
    }
  }

  /// Serialize this input into the object shape emitted by JavaScript PostCSS
  /// when calling `Input#toJSON()`.
  pub fn to_json_value(&self) -> JsonValue {
    let mut object = JsonMap::new();
    object.insert("css".to_string(), JsonValue::String(self.css.clone()));
    if self.has_bom {
      object.insert("hasBOM".to_string(), JsonValue::Bool(true));
    }
    if let Some(file) = &self.file {
      object.insert("file".to_string(), JsonValue::String(file.clone()));
    }
    if let Some(id) = &self.id {
      object.insert("id".to_string(), JsonValue::String(id.clone()));
    }
    if let Some(map) = &self.map {
      object.insert("map".to_string(), map.to_json_value());
    }
    JsonValue::Object(object)
  }
}

impl From<String> for Input {
  fn from(css: String) -> Self {
    Input::new(css, InputOptions::default()).expect("valid input")
  }
}

#[derive(Clone)]
pub struct InputRef(Rc<Input>);

impl InputRef {
  pub fn new(input: Input) -> Self {
    InputRef(Rc::new(input))
  }

  pub fn clone_inner(&self) -> Rc<Input> {
    self.0.clone()
  }

  pub fn to_json_value(&self) -> JsonValue {
    self.0.to_json_value()
  }

  pub fn as_ptr(&self) -> *const Input {
    Rc::as_ptr(&self.0)
  }
}

impl std::ops::Deref for InputRef {
  type Target = Input;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl From<Input> for InputRef {
  fn from(input: Input) -> Self {
    InputRef::new(input)
  }
}

impl From<&Input> for InputRef {
  fn from(input: &Input) -> Self {
    InputRef::new(input.clone())
  }
}

impl std::fmt::Debug for InputRef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("InputRef")
      .field("file", &self.file)
      .field("from", &self.from)
      .finish()
  }
}
