use std::path::Path;

pub use atlaspack_vcs::{FailureMode, VCSState};
use napi::{Env, JsUnknown};
use napi_derive::napi;

#[napi]
pub fn get_vcs_state_snapshot(
  env: Env,
  path: String,
  exclude_patterns: Vec<String>,
) -> napi::Result<JsUnknown> {
  let path = Path::new(&path);
  let vcs_state = VCSState::read_from_repository(
    path,
    &exclude_patterns,
    FailureMode::IgnoreMissingNodeModules,
  )
  .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?;
  let vcs_state = env.to_js_value(&vcs_state)?;

  Ok(vcs_state)
}

#[napi]
pub fn get_events_since(
  repo_path: String,
  old_rev: String,
  new_rev: Option<String>,
) -> napi::Result<Vec<String>> {
  let repo_path = Path::new(&repo_path);
  let files = atlaspack_vcs::get_changed_files(
    repo_path,
    &old_rev,
    new_rev.as_deref().unwrap_or("HEAD"),
    FailureMode::IgnoreMissingNodeModules,
  )
  .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?;

  let files = files
    .iter()
    .map(|file| file.to_str().unwrap().to_string())
    .collect();

  Ok(files)
}
