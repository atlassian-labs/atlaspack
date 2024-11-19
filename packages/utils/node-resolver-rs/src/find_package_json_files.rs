use std::io;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn find_package_json_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
  let mut package_json_files = Vec::new();
  let mut stack = vec![dir.to_path_buf()];

  while let Some(current_dir) = stack.pop() {
    let mut entries = fs::read_dir(&current_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
      let path = entry.path();

      if path.is_dir() {
        stack.push(path);
      } else if path.is_file() && path.file_name() == Some("package.json".as_ref()) {
        package_json_files.push(path);
      }
    }
  }

  Ok(package_json_files)
}
