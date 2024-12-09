use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::types::JSONObject;

use super::CodeFrame;

/// This is a user facing error for Atlaspack.
///
/// Usually but not always this is linked to a source-code location.
#[derive(Error, Debug, Deserialize, PartialEq, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
  /// A summary user-facing message
  pub message: String,

  /// Indicates where this diagnostic was emitted from
  ///
  /// Consumers can also enable backtraces for more detailed origin information.
  pub origin: Option<String>,

  /// A stacktrace of the error (optional)
  pub stack: Option<String>,

  /// Name of the error (optional)
  pub name: Option<String>,

  /// A list of files with source-code highlights
  pub code_frames: Option<Vec<CodeFrame>>,

  /// Hints for the user
  pub hints: Option<Vec<String>>,

  /// Skip formatting the code in this error
  pub skip_formatting: bool,

  /// URL for the user to refer to documentation
  #[serde(rename = "documentationURL")]
  pub documentation_url: Option<String>,

  /// Diagnostic specific metadata (optional)
  pub meta: Option<JSONObject>,
}

impl Display for Diagnostic {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.message)
  }
}

impl Diagnostic {
  pub fn name_matches<N: AsRef<str>>(&self, name: N) -> bool {
    if self.name.as_ref().is_some_and(|n| n == name.as_ref()) {
      return true;
    }
    return false;
  }
}
