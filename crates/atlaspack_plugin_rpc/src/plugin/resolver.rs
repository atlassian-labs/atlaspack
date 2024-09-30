use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;

use atlaspack_config::PluginNode;
use atlaspack_core::hash::IdentifierHasher;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::Resolved;
use atlaspack_core::plugin::ResolverPlugin;

use crate::LoadPluginKind;
use crate::LoadPluginOptions;
use crate::RpcWorkerRef;
use crate::RunResolverResolve;

pub struct RpcResolverPlugin {
  resolve_from: PathBuf,
  specifier: String,
  started: bool,
  rpc_worker: RpcWorkerRef,
}

impl Debug for RpcResolverPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcResolverPlugin")
  }
}

impl RpcResolverPlugin {
  pub fn new(
    _ctx: &PluginContext,
    plugin: &PluginNode,
    rpc_worker: RpcWorkerRef,
  ) -> Result<Self, anyhow::Error> {
    Ok(RpcResolverPlugin {
      resolve_from: plugin.resolve_from.to_path_buf(),
      specifier: plugin.package_name.clone(),
      rpc_worker,
      started: false,
    })
  }
}

impl ResolverPlugin for RpcResolverPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    self.resolve_from.hash(&mut hasher);
    self.specifier.hash(&mut hasher);
    self.started.hash(&mut hasher);
    hasher.finish()
  }

  fn resolve(&self, ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
    if !self.started {
      self.rpc_worker.load_plugin(LoadPluginOptions {
        kind: LoadPluginKind::Resolver,
        specifier: self.specifier.clone(),
        resolve_from: self.resolve_from.clone(),
      })?;
    }

    self.rpc_worker.run_resolver_resolve(RunResolverResolve {
      key: self.specifier.clone(),
      dependency: (&*ctx.dependency).clone(),
      specifier: (&*ctx.specifier).to_owned(),
    })
  }
}
