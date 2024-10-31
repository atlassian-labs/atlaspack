use std::path::{Path, PathBuf};

use atlaspack_filesystem::FileSystemRef;
use parcel_sourcemap::SourceMapError;

use crate::SourceMap;

pub fn load_sourcemap_url(
  fs: &FileSystemRef,
  project_root: &Path,
  file_path: &Path,
  url: &str,
) -> Result<SourceMap, SourceMapError> {
  let protocol = url.split_once(":");
  if let Some(("data", _)) = protocol {
    return SourceMap::from_data_url(project_root, url);
  }

  let mut source_map_path = PathBuf::from(if let Some(("file", _)) = protocol {
    url.replace("file://", "")
  } else {
    url.to_string()
  });

  if !source_map_path.is_absolute() {
    source_map_path = file_path
      .parent()
      .unwrap_or(&PathBuf::default())
      .join(source_map_path)
  }

  let json = fs.read_to_string(&source_map_path)?;

  SourceMap::from_json(project_root, &json)
}
