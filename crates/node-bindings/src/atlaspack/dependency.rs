use atlaspack_core::types::{
  AssetId, BundleBehavior, Environment, Priority, SpecifierType, Target,
};
use atlaspack_resolver::ExportsCondition;
use napi::{Env, JsUnknown};
use napi_derive::napi;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DependencyIdParams {
  source_asset_id: Option<AssetId>,
  specifier: String,
  env: Environment,
  target: Option<Target>,
  pipeline: Option<String>,
  specifier_type: Option<SpecifierType>,
  bundle_behavior: Option<BundleBehavior>,
  priority: Option<Priority>,
  package_conditions: Option<ExportsCondition>,
}

#[napi]
fn create_dependency_id(env: Env, params: JsUnknown) -> napi::Result<String> {
  let params = env.from_js_value(params)?;

  let DependencyIdParams {
    source_asset_id,
    specifier,
    env,
    target,
    pipeline,
    specifier_type,
    bundle_behavior,
    priority,
    package_conditions,
  } = params;

  Ok(atlaspack_core::types::create_dependency_id(
    source_asset_id.as_ref(),
    &specifier,
    &env,
    target.as_ref(),
    pipeline.as_ref().map(|s| s.as_str()),
    &specifier_type.unwrap_or_default(),
    &bundle_behavior.unwrap_or_default(),
    &priority.unwrap_or_default(),
    &package_conditions.unwrap_or_default(),
  ))
}
