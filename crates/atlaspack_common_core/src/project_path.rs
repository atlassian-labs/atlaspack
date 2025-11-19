use std::path::{Path, PathBuf};

/// Converts the path so that it is relative to the project root
pub fn to_project_path(project_root: &Path, file_path: &Path) -> PathBuf {
  file_path
    .strip_prefix(project_root)
    .unwrap_or(file_path)
    .to_path_buf()
}

#[cfg(test)]
mod tests {
  use super::*;

  mod to_project_path {
    use super::*;

    #[test]
    fn returns_file_path_when_outside_project_root() {
      let file_path = Path::new("test").join("a.js");

      assert_eq!(
        to_project_path(Path::new("project-root"), &file_path),
        file_path
      );
    }

    #[test]
    fn returns_project_path_when_inside_project_root() {
      let project_root = Path::new("project-root");
      let project_path = Path::new("test").join("a.js");

      assert_eq!(
        to_project_path(project_root, &project_root.join(project_path.clone())),
        project_path
      );
    }
  }
}
