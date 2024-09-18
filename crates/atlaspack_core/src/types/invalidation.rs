use std::path::PathBuf;

#[derive(Debug, PartialEq, Clone)]
pub enum Invalidation {
  FileChange(PathBuf),
}
