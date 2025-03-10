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

#[napi(object)]
pub struct NodeChangeEvent {
  pub path: String,
  pub change_type: String,
}

#[napi(object)]
pub struct NodeVCSFile {
  pub path: String,
  pub hash: String,
}

#[napi]
pub fn get_events_since(
  env: Env,
  repo_path: String,
  vcs_state_snapshot: JsUnknown,
  new_rev: Option<String>,
) -> napi::Result<Vec<NodeChangeEvent>> {
  let repo_path = Path::new(&repo_path);
  let vcs_state = env.from_js_value::<VCSState, _>(vcs_state_snapshot)?;
  let files = atlaspack_vcs::get_changed_files(
    repo_path,
    &vcs_state,
    new_rev.as_deref(),
    FailureMode::IgnoreMissingNodeModules,
  )
  .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?;

  let files = files
    .iter()
    .map(|event| NodeChangeEvent {
      path: event.path().to_str().unwrap().to_string(),
      change_type: event.change_type_str().to_string(),
    })
    .collect();

  Ok(files)
}
