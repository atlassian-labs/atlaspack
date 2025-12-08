use std::path::Path;

/// Returns the relative path from `from` to `path` as a `String`.
/// e.g. for path="/a/b/c/d.txt" and from="/a/b", returns "c/d.txt"
/// and for path="/a/b/c/d.txt" and from="/a/b/e", returns "../c/d.txt"
///
/// Optimized for performance when called frequently (e.g., 100k times per build).
pub fn relative_path(path: &Path, from: &Path) -> String {
  // Collect components once - unavoidable allocation for comparison
  let path_components: Vec<_> = path.components().collect();
  let from_components: Vec<_> = from.components().collect();

  // Find common prefix length
  let common_len = path_components
    .iter()
    .zip(from_components.iter())
    .take_while(|(a, b)| a == b)
    .count();

  let ups = from_components.len() - common_len;
  let remaining = &path_components[common_len..];

  // Early return for same path
  if ups == 0 && remaining.is_empty() {
    return ".".to_string();
  }

  // Pre-calculate capacity to avoid string reallocations
  let mut capacity = 0;
  if ups > 0 {
    capacity += ups * 3; // "../" for each up
    if ups > 1 {
      capacity -= 1; // one less separator
    }
  }
  if !remaining.is_empty() {
    if ups > 0 {
      capacity += 1; // separator between ups and remaining
    }
    for (i, component) in remaining.iter().enumerate() {
      if let Some(name) = component.as_os_str().to_str() {
        capacity += name.len();
        if i < remaining.len() - 1 {
          capacity += 1; // separator
        }
      }
    }
  }

  let mut result = String::with_capacity(capacity);

  // Add ".." components - build string directly to avoid intermediate allocations
  for i in 0..ups {
    if i > 0 {
      result.push('/');
    }
    result.push_str("..");
  }

  // Add remaining path components
  for (i, component) in remaining.iter().enumerate() {
    if ups > 0 || i > 0 {
      result.push('/');
    }
    if let Some(name) = component.as_os_str().to_str() {
      result.push_str(name);
    }
  }

  result
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;

  #[test]
  fn test_relative_path_child_directory() {
    let path = Path::new("/a/b/c/d.txt");
    let from = Path::new("/a/b");
    assert_eq!(relative_path(path, from), "c/d.txt");
  }

  #[test]
  fn test_relative_path_sibling_directory() {
    let path = Path::new("/a/b/c/d.txt");
    let from = Path::new("/a/b/e");
    assert_eq!(relative_path(path, from), "../c/d.txt");
  }

  #[test]
  fn test_relative_path_same_directory() {
    let path = Path::new("/a/b/file.txt");
    let from = Path::new("/a/b");
    assert_eq!(relative_path(path, from), "file.txt");
  }

  #[test]
  fn test_relative_path_same_file() {
    let path = Path::new("/a/b/file.txt");
    let from = Path::new("/a/b/file.txt");
    assert_eq!(relative_path(path, from), ".");
  }

  #[test]
  fn test_relative_path_parent_directory() {
    let path = Path::new("/a/b");
    let from = Path::new("/a/b/c");
    assert_eq!(relative_path(path, from), "..");
  }

  #[test]
  fn test_relative_path_multiple_levels_up() {
    let path = Path::new("/a/x/y.txt");
    let from = Path::new("/a/b/c/d");
    assert_eq!(relative_path(path, from), "../../../x/y.txt");
  }

  #[test]
  fn test_relative_path_no_common_prefix() {
    let path = Path::new("/x/y/z.txt");
    let from = Path::new("/a/b/c");
    assert_eq!(relative_path(path, from), "../../../x/y/z.txt");
  }

  #[test]
  fn test_relative_path_root() {
    let path = Path::new("/file.txt");
    let from = Path::new("/");
    assert_eq!(relative_path(path, from), "file.txt");
  }

  #[test]
  fn test_relative_path_from_root() {
    let path = Path::new("/");
    let from = Path::new("/a/b");
    assert_eq!(relative_path(path, from), "../..");
  }
}
