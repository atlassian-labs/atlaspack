use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use parking_lot::RwLock;

pub fn canonicalize_impl(
  current_working_directory: &RwLock<PathBuf>,
  path: &Path,
) -> PathBuf {
  let cwd = current_working_directory.read();
  let mut result = if path.is_absolute() {
    vec![]
  } else {
    cwd.components().collect()
  };

  let components = path.components();
  for component in components {
    match component {
      Component::Prefix(prefix) => {
        result = vec![Component::Prefix(prefix)];
      }
      Component::RootDir => {
        result.push(Component::RootDir);
      }
      Component::CurDir => {}
      Component::ParentDir => {
        result.pop();
      }
      Component::Normal(path) => {
        result.push(Component::Normal(path));
      }
    }
  }

  PathBuf::from_iter(result)
}
