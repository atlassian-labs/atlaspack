use atlaspack_core::types::{
  engines::Engines, Environment, EnvironmentContext, OutputFormat, SourceType,
  TargetSourceMapOptions,
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

static ENVIRONMENT_MANAGER: std::sync::OnceLock<EnvironmentManager> = std::sync::OnceLock::new();

struct EnvironmentManager {
  environments: std::sync::RwLock<std::collections::HashMap<String, Environment>>,
}

impl EnvironmentManager {
  pub fn new() -> Self {
    Self {
      environments: std::sync::RwLock::new(std::collections::HashMap::new()),
    }
  }
}

#[napi]
pub fn get_environment(env: Env, id: String) -> napi::Result<JsUnknown> {
  let manager = ENVIRONMENT_MANAGER.get_or_init(EnvironmentManager::new);
  let environments = manager.environments.read().unwrap();
  let environment = environments.get(&id).ok_or(napi::Error::new(
    napi::Status::GenericFailure,
    format!("Environment with id {} not found", id),
  ))?;

  let unknown = env.to_js_value(&environment)?;

  let mut obj = unknown.coerce_to_object()?;
  obj.set("id", environment.id())?;
  Ok(obj.into_unknown())
}

#[napi]
pub fn add_environment(env: Env, environment: JsUnknown) -> napi::Result<()> {
  let environment: Environment = env.from_js_value(&environment)?;
  let manager = ENVIRONMENT_MANAGER.get_or_init(EnvironmentManager::new);
  let mut envs = manager.environments.write().unwrap();
  envs.insert(environment.id(), environment);

  Ok(())
}
