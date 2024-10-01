use std::path::{Path, PathBuf};

use atlaspack_core::types::{Location, SourceLocation};

pub(crate) fn convert_loc(
  project_root: &Path,
  file_path: PathBuf,
  loc: &atlaspack_js_swc_core::SourceLocation,
) -> SourceLocation {
  SourceLocation {
    file_path: file_path
      .strip_prefix(project_root)
      .map(Path::to_path_buf)
      .unwrap_or_else(|_| file_path),
    start: Location {
      line: loc.start_line,
      column: loc.start_col,
    },
    end: Location {
      line: loc.end_line,
      column: loc.end_col,
    },
  }
}
