use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_config::PluginNode;
use atlaspack_core::hash::IdentifierHasher;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::Resolved;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::types::Dependency;
use atlaspack_napi_helpers::anyhow_from_napi;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use super::super::rpc::nodejs_rpc_worker_farm::NodeJsWorkerCollection;
use super::super::rpc::LoadPluginKind;
use super::super::rpc::LoadPluginOptions;
use super::plugin_options::RpcPluginOptions;

pub struct RpcNodejsResolverPlugin {
  plugin_options: RpcPluginOptions,
  rpc_workers: Arc<NodeJsWorkerCollection>,
  resolve_from: PathBuf,
  specifier: String,
  started: OnceCell<Vec<()>>,
}

impl Debug for RpcNodejsResolverPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcNodejsResolverPlugin")
  }
}

impl RpcNodejsResolverPlugin {
  pub fn new(
    rpc_workers: Arc<NodeJsWorkerCollection>,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> Result<Self, anyhow::Error> {
    let plugin_options = RpcPluginOptions {
      hmr_options: None,
      project_root: ctx.options.project_root.clone(),
      mode: ctx.options.mode.clone(),
    };
    Ok(Self {
      plugin_options,
      resolve_from: plugin.resolve_from.to_path_buf(),
      specifier: plugin.package_name.clone(),
      rpc_workers,
      started: OnceCell::new(),
    })
  }
}

impl ResolverPlugin for RpcNodejsResolverPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    self.resolve_from.hash(&mut hasher);
    self.specifier.hash(&mut hasher);
    hasher.finish()
  }

  fn resolve(&self, ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
    self.started.get_or_try_init(|| {
      self.rpc_workers.exec_on_all(|worker| {
        worker.load_plugin(LoadPluginOptions {
          kind: LoadPluginKind::Resolver,
          specifier: self.specifier.clone(),
          resolve_from: self.resolve_from.clone(),
        })
      })
    })?;

    self.rpc_workers.exec_on_one(|worker| {
      worker
        .run_resolver_resolve_fn
        .call_with_return_serde(RunResolverResolve {
          key: self.specifier.clone(),
          dependency: (&*ctx.dependency).clone(),
          specifier: (&*ctx.specifier).to_owned(),
          pipeline: ctx.pipeline.clone(),
          options: self.plugin_options.clone(),
        })
        .map_err(anyhow_from_napi)
    })
  }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResolverResolve {
  pub key: String,
  pub dependency: Dependency,
  pub specifier: String,
  pub pipeline: Option<String>,
  pub options: RpcPluginOptions,
}
