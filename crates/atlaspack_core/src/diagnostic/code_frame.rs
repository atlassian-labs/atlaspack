use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::config_loader::ConfigFile;
use crate::types::File;
use crate::types::FileType;

use super::CodeHighlight;
use super::Language;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeFrame {
  /// Source-code of the file at the time of error
  // TODO: might want to RC or intern
  pub code: Option<String>,

  /// Path to the source file if applicable.
  // TODO: In the future we might need to discern between errors on a source file in disk or in-memory.
  pub file_path: Option<PathBuf>,

  /// The language associated with the code
  pub language: Option<Language>,

  /// List of source-code highlight messages
  pub code_highlights: Vec<CodeHighlight>,
}

impl From<File> for CodeFrame {
  fn from(file: File) -> Self {
    let language = file
      .path
      .extension()
      .map(|ext| Language(FileType::from_extension(&ext.to_string_lossy())));

    CodeFrame {
      code: Some(file.contents),
      code_highlights: Vec::new(),
      language,
      file_path: Some(file.path),
    }
  }
}

impl<T> From<&ConfigFile<T>> for CodeFrame {
  fn from(file: &ConfigFile<T>) -> Self {
    CodeFrame::from(File {
      contents: file.raw.clone(),
      path: file.path.clone(),
    })
  }
}

impl From<PathBuf> for CodeFrame {
  fn from(path: PathBuf) -> Self {
    let language = path
      .extension()
      .map(|ext| Language(FileType::from_extension(&ext.to_string_lossy())));

    CodeFrame {
      code: None,
      code_highlights: Vec::new(),
      language,
      file_path: Some(path),
    }
  }
}
