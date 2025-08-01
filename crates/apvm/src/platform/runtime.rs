use std::path::PathBuf;

pub fn resolve_executable<S: AsRef<str>>(executable: S) -> anyhow::Result<PathBuf> {
  let runtime = if executable.as_ref().starts_with("/") {
    PathBuf::from(executable.as_ref())
  } else {
    which::CanonicalPath::new(executable.as_ref())?.to_path_buf()
  };
  if !std::fs::exists(&runtime)? {
    return Err(anyhow::anyhow!("Cannot find runtime executable"));
  }
  Ok(runtime)
}
