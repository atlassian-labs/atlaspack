use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
  Npm,
  Git,
  Local,
  Release,
  Unmanaged,
}
