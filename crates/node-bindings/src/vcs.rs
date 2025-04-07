use std::path::Path;

pub use atlaspack_vcs::{FailureMode, VCSState};
use napi::{Env, JsObject, JsUnknown};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Run a function in a background one-off thread and return a promise to the result.
fn run_in_background<T, F>(env: Env, run: F) -> napi::Result<JsObject>
where
  T: Serialize + Send + 'static,
  F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
  let (deferred, promise) = env.create_deferred()?;

  std::thread::spawn(move || match run() {
    Ok(result) => {
      deferred.resolve(move |env| {
        let result = env.to_js_value(&result)?;
        Ok(result)
      });
    }
    Err(err) => {
      deferred.reject(napi::Error::new(
        napi::Status::GenericFailure,
        format!("[napi] {}", err),
      ));
    }
  });

  Ok(promise)
}

#[napi]
pub fn get_vcs_state_snapshot(
  env: Env,
  path: String,
  exclude_patterns: Vec<String>,
) -> napi::Result<JsObject> {
  run_in_background(env, move || {
    let path = Path::new(&path);
    VCSState::read_from_repository(
      path,
      &exclude_patterns,
      FailureMode::IgnoreMissingNodeModules,
    )
  })
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeChangeEvent {
  pub path: String,
  pub change_type: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

  run_in_background(env, move || {
    let repo_path = Path::new(&repo_path);
    let files = atlaspack_vcs::get_changed_files(
      repo_path,
      &vcs_state,
      new_rev.as_deref(),
      FailureMode::IgnoreMissingNodeModules,
    )?;

    let files: Vec<NodeChangeEvent> = files
      .iter()
      .map(|event| NodeChangeEvent {
        path: event.path().to_str().unwrap().to_string(),
        change_type: event.change_type_str().to_string(),
      })
      .collect();

    Ok(files)
  })
}
