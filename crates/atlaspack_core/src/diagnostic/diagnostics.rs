use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use super::Diagnostic;

#[derive(Error, Default, Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
  pub fn as_ref(&self) -> &Vec<Diagnostic> {
    &self.0
  }

  pub fn as_mut(&mut self) -> &mut Vec<Diagnostic> {
    &mut self.0
  }

  pub fn into_inner(self) -> Vec<Diagnostic> {
    self.0
  }
}

impl Display for Diagnostics {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let mut output = String::new();
    for diagnostic in &self.0 {
      output += &format!("{}\n", diagnostic);
    }
    write!(f, "{}", output)
  }
}

impl Serialize for Diagnostics {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    self.0.serialize(serializer)
  }
}

impl From<Vec<Diagnostic>> for Diagnostics {
  fn from(diagnostics: Vec<Diagnostic>) -> Self {
    Diagnostics(diagnostics)
  }
}

impl From<Diagnostic> for Diagnostics {
  fn from(diagnostic: Diagnostic) -> Self {
    Diagnostics(vec![diagnostic])
  }
}
