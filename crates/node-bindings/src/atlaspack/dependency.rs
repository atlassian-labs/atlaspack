use atlaspack_core::types::{AssetId, BundleBehavior, Priority, SpecifierType, Target};
use atlaspack_resolver::ExportsCondition;
use napi::{Env, JsUnknown};
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Permissive parsing of bundle behavior that works with both u32 and string
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum JSBundleBehavior {
  String(BundleBehaviorStr),
  Int(BundleBehavior),
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum BundleBehaviorStr {
  Inline,
  Isolated,
}

impl From<JSBundleBehavior> for BundleBehavior {
  fn from(value: JSBundleBehavior) -> Self {
    match value {
      JSBundleBehavior::String(BundleBehaviorStr::Inline) => BundleBehavior::Inline,
      JSBundleBehavior::String(BundleBehaviorStr::Isolated) => BundleBehavior::Isolated,
      JSBundleBehavior::Int(value) => value,
    }
  }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DependencyIdParams {
  source_asset_id: Option<AssetId>,
  specifier: String,
  environment_id: String,
  target: Option<Target>,
  pipeline: Option<String>,
  specifier_type: Option<SpecifierType>,
  bundle_behavior: Option<JSBundleBehavior>,
  priority: Option<Priority>,
  package_conditions: Option<ExportsCondition>,
}

#[napi]
fn create_dependency_id(env: Env, params: JsUnknown) -> napi::Result<String> {
  let params: DependencyIdParams = env.from_js_value(params)?;

  let DependencyIdParams {
    source_asset_id,
    specifier,
    environment_id,
    target,
    pipeline,
    specifier_type,
    bundle_behavior,
    priority,
    package_conditions,
  } = params;

  let bundle_behavior = bundle_behavior.map(|b| -> BundleBehavior { b.into() });

  Ok(atlaspack_core::types::create_dependency_id(
    source_asset_id.as_ref(),
    &specifier,
    &environment_id,
    target.as_ref(),
    pipeline.as_deref(),
    &specifier_type.unwrap_or_default(),
    &bundle_behavior,
    &priority.unwrap_or_default(),
    &package_conditions.unwrap_or_default(),
  ))
}
