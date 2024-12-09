use serde::Deserialize;
use serde::Serialize;

use crate::types::FileType;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Default)]
pub struct Language(pub(super) FileType);

impl From<FileType> for Language {
  fn from(value: FileType) -> Self {
    Self(value)
  }
}
