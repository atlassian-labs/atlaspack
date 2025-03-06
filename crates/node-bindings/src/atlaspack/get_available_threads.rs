use napi::{Env, JsNumber};
use napi_derive::napi;

#[napi]
pub fn get_available_threads(env: Env) -> napi::Result<JsNumber> {
  env.create_uint32(num_cpus::get() as u32)
}
