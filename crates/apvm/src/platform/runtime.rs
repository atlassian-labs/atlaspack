use std::path::PathBuf;

#[allow(unused)]
pub fn resolve_runtime<S: AsRef<str>>(runtime: S) -> anyhow::Result<PathBuf> {
  let runtime = if runtime.as_ref().starts_with("/") {
    PathBuf::from(runtime.as_ref())
  } else {
    which::CanonicalPath::new(runtime.as_ref())?.to_path_buf()
  };
  if !std::fs::exists(&runtime)? {
    return Err(anyhow::anyhow!("Cannot find runtime executable"));
  }
  Ok(runtime)
}
