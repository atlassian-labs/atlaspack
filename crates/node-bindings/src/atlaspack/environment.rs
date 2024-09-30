use atlaspack_core::types::{
  engines::Engines, EnvironmentContext, OutputFormat, SourceType, TargetSourceMapOptions,
};
use atlaspack_resolver::IncludeNodeModules;
use napi::{Env, JsUnknown};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EnvironmentIdParams {
  #[serde(default)]
  context: EnvironmentContext,
  #[serde(default)]
  engines: Engines,
  #[serde(default)]
  include_node_modules: IncludeNodeModules,
  #[serde(default)]
  output_format: OutputFormat,
  #[serde(default)]
  source_type: SourceType,
  #[serde(default)]
  is_library: bool,
  #[serde(default)]
  should_optimize: bool,
  #[serde(default)]
  should_scope_hoist: bool,
  #[serde(default)]
  source_map: Option<TargetSourceMapOptions>,
}

#[napi]
pub fn create_environment_id(env: Env, params: JsUnknown) -> napi::Result<String> {
  let params: EnvironmentIdParams = env.from_js_value(params)?;
  let EnvironmentIdParams {
    context,
    engines,
    include_node_modules,
    output_format,
    source_type,
    is_library,
    should_optimize,
    should_scope_hoist,
    source_map,
  } = params;

  Ok(atlaspack_core::types::create_environment_id(
    &context,
    &engines,
    &include_node_modules,
    &output_format,
    &source_type,
    &is_library,
    &should_optimize,
    &should_scope_hoist,
    &source_map,
  ))
}
