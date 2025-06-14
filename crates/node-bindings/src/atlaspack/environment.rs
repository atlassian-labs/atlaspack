use atlaspack_core::types::{
  engines::Engines, Environment, EnvironmentContext, OutputFormat, SourceType,
  TargetSourceMapOptions,
};
use atlaspack_resolver::IncludeNodeModules;
use napi::{Env, JsUnknown};
use napi_derive::napi;
use parking_lot::RwLock;
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
  environments: RwLock<std::collections::HashMap<String, Environment>>,
}

impl EnvironmentManager {
  pub fn new() -> Self {
    Self {
      environments: RwLock::new(std::collections::HashMap::new()),
    }
  }

  pub fn get_environment(&self, id: &str) -> Option<Environment> {
    let environments = self.environments.read();
    environments.get(id).cloned()
  }

  pub fn add_environment(&self, environment: Environment) {
    let mut environments = self.environments.write();
    environments.insert(environment.id(), environment);
  }

  pub fn set_all_environments(&self, new_environments: Vec<Environment>) {
    let mut environments = self.environments.write();
    environments.clear();
    for environment in new_environments {
      environments.insert(environment.id(), environment);
    }
    drop(environments);
  }

  pub fn get_all_environments(&self) -> Vec<Environment> {
    let environments = self.environments.read();
    environments.values().cloned().collect()
  }
}

/// Overwrite all environments with a new set of environments
#[napi]
pub fn set_all_environments(env: Env, environments: JsUnknown) -> napi::Result<()> {
  let manager = ENVIRONMENT_MANAGER.get_or_init(EnvironmentManager::new);

  let environments: Vec<Environment> = env.from_js_value(&environments)?;
  manager.set_all_environments(environments);

  Ok(())
}

/// Get an array of all environments
#[napi]
pub fn get_all_environments(env: Env) -> napi::Result<Vec<JsUnknown>> {
  let manager = ENVIRONMENT_MANAGER.get_or_init(EnvironmentManager::new);
  let environments = manager.get_all_environments();

  let mut result = Vec::new();
  for environment in environments {
    let unknown = env.to_js_value(&environment)?;
    let mut obj = unknown.coerce_to_object()?;
    obj.set("id", environment.id())?;
    result.push(obj.into_unknown());
  }

  Ok(result)
}

/// Get environment by ID
#[napi]
pub fn get_environment(env: Env, id: String) -> napi::Result<JsUnknown> {
  let manager = ENVIRONMENT_MANAGER.get_or_init(EnvironmentManager::new);

  let environment = manager.get_environment(&id).ok_or(napi::Error::new(
    napi::Status::GenericFailure,
    format!("Environment with id {} not found", id),
  ))?;

  let unknown = env.to_js_value(&environment)?;
  let mut obj = unknown.coerce_to_object()?;
  obj.set("id", environment.id())?;
  Ok(obj.into_unknown())
}

/// Add an environment to the global manager
#[napi]
pub fn add_environment(env: Env, environment: JsUnknown) -> napi::Result<()> {
  let environment: Environment = env.from_js_value(&environment)?;
  let manager = ENVIRONMENT_MANAGER.get_or_init(EnvironmentManager::new);
  manager.add_environment(environment);
  Ok(())
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_environment_manager() {
    let manager = EnvironmentManager::new();
    let environment = Environment::default();
    manager.add_environment(environment.clone());
    let stored_environment = manager.get_environment(&environment.id()).unwrap();
    assert_eq!(environment, stored_environment);
  }
}
