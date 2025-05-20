use atlaspack_dev_server::{run_server, Options, ServerHandle};
use napi::{bindgen_prelude::External, Env, JsFunction};
use napi_derive::napi;

#[napi]
pub async fn start_dev_server(
  env: Env,
  dist_dir: String,
  // path: string -> Promise<void>
  request_bundle_build: JsFunction,
) -> anyhow::Result<External<ServerHandle>> {
  let thread_safe_request_bundle_build =
    env.create_threadsafe_function(&request_bundle_build, 5, || {})?;

  let options = Options { dist_dir };
  let handle = run_server(options).await;
  External::new(handle)
}

#[napi]
pub fn dev_server_stop(handle: External<ServerHandle>) {
  handle.as_ref().stop();
}

#[napi]
pub fn dev_server_on_build_finished(handle: External<ServerHandle>) {
  handle.as_ref().on_build_finished();
}

#[napi]
pub fn dev_server_on_build_started(handle: External<ServerHandle>) {
  handle.as_ref().on_build_started();
}
