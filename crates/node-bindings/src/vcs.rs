use std::path::Path;

pub use atlaspack_vcs::{FailureMode, VCSState};
use napi::{Env, JsObject, JsUnknown};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[napi]
pub fn get_vcs_state_snapshot(
  env: Env,
  path: String,
  exclude_patterns: Vec<String>,
) -> napi::Result<JsObject> {
  env.execute_tokio_future(
    async move {
      let vcs_state = tokio::task::spawn_blocking(move || -> anyhow::Result<VCSState> {
        let path = Path::new(&path);
        let vcs_state = VCSState::read_from_repository(
          path,
          &exclude_patterns,
          FailureMode::IgnoreMissingNodeModules,
        )?;
        Ok(vcs_state)
      })
      .await
      .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?
      .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?;

      Ok(vcs_state)
    },
    |&mut env, vcs_state| {
      let vcs_state = env.to_js_value(&vcs_state)?;
      Ok(vcs_state)
    },
  )
}

#[derive(Serialize, Deserialize)]
pub struct NodeChangeEvent {
  pub path: String,
  pub change_type: String,
}

#[derive(Serialize, Deserialize)]
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
) -> napi::Result<JsObject> {
  let vcs_state = env.from_js_value::<VCSState, _>(vcs_state_snapshot)?;

  env.execute_tokio_future(
    async move {
      let files = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<NodeChangeEvent>> {
        let repo_path = Path::new(&repo_path);
        let files = atlaspack_vcs::get_changed_files(
          repo_path,
          &vcs_state,
          new_rev.as_deref(),
          FailureMode::IgnoreMissingNodeModules,
        )?;

        let files = files
          .iter()
          .map(|event| NodeChangeEvent {
            path: event.path().to_str().unwrap().to_string(),
            change_type: event.change_type_str().to_string(),
          })
          .collect();

        Ok(files)
      })
      .await
      .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?
      .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("[napi] {}", err)))?;

      Ok(files)
    },
    |&mut env, files| {
      let files = env.to_js_value(&files);
      Ok(files)
    },
  )
}
