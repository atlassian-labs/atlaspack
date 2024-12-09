use thiserror::Error;

use crate::diagnostic::Diagnostic;
use crate::diagnostic::Diagnostics;

pub type AtlaspackResult<T> = std::result::Result<T, AtlaspackError>;

#[derive(Error, Debug)]
pub enum AtlaspackError {
  #[error("{}", .0)]
  Io(#[from] std::io::Error),

  #[error("{}", .0)]
  Diagnostic(#[from] Diagnostic),

  #[error("{}", .0)]
  Diagnostics(#[from] Diagnostics),

  #[error("{}", .0)]
  Message(String),

  #[error("{}", .0)]
  Unknown(anyhow::Error),
}

impl AtlaspackError {
  pub fn diagnostic_name_matches<N: AsRef<str>>(&self, name: N) -> bool {
    let Self::Diagnostic(diagnostic) = self else {
      return false;
    };
    if diagnostic.name.as_ref().is_some_and(|n| n == name.as_ref()) {
      return true;
    }
    return false;
  }
}
