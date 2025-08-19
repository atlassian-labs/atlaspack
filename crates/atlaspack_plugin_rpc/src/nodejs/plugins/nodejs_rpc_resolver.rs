use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::hash::IdentifierHasher;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::PluginOptions;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::Resolved;
use atlaspack_core::plugin::ResolverPlugin;
use tokio::sync::OnceCell;

use crate::javascript_plugin_api::JavaScriptPluginAPI;
use crate::javascript_plugin_api::RunResolverResolve;

use crate::javascript_plugin_api::plugin_options::RpcPluginOptions;
use crate::javascript_plugin_api::LoadPluginKind;
use crate::javascript_plugin_api::LoadPluginOptions;

/// Plugin state once initialized
struct InitializedState {
  rpc_plugin_options: RpcPluginOptions,
}

pub struct RpcNodejsResolverPlugin {
  nodejs_workers: Arc<dyn JavaScriptPluginAPI>,
  plugin_options: Arc<PluginOptions>,
  plugin_node: Arc<PluginNode>,
  started: OnceCell<InitializedState>,
}

impl Debug for RpcNodejsResolverPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "RpcNodejsResolverPlugin({})",
      self.plugin_node.package_name
    )
  }
}

impl RpcNodejsResolverPlugin {
  pub fn new(
    nodejs_workers: Arc<dyn JavaScriptPluginAPI>,
    ctx: &PluginContext,
    plugin_node: &PluginNode,
  ) -> Result<Self, anyhow::Error> {
    Ok(Self {
      nodejs_workers,
      plugin_options: ctx.options.clone(),
      plugin_node: Arc::new(plugin_node.clone()),
      started: OnceCell::new(),
    })
  }

  async fn get_or_init_state(&self) -> anyhow::Result<&InitializedState> {
    self
      .started
      .get_or_try_init::<anyhow::Error, _, _>(|| async move {
        self
          .nodejs_workers
          .load_plugin(LoadPluginOptions {
            kind: LoadPluginKind::Resolver,
            specifier: self.plugin_node.package_name.clone(),
            #[allow(clippy::needless_borrow)]
            resolve_from: (&*self.plugin_node.resolve_from).clone(),
          })
          .await?;

        Ok(InitializedState {
          rpc_plugin_options: RpcPluginOptions {
            hmr_options: None,
            project_root: self.plugin_options.project_root.clone(),
            mode: self.plugin_options.mode.clone(),
          },
        })
      })
      .await
  }
}

#[async_trait]
impl ResolverPlugin for RpcNodejsResolverPlugin {
  fn id(&self) -> u64 {
    let mut hasher = IdentifierHasher::new();
    self.type_id().hash(&mut hasher);
    self.plugin_node.resolve_from.hash(&mut hasher);
    self.plugin_node.package_name.hash(&mut hasher);
    hasher.finish()
  }

  async fn resolve(&self, ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
    let state = self.get_or_init_state().await?;

    self
      .nodejs_workers
      .resolve(RunResolverResolve {
        key: self.plugin_node.package_name.clone(),
        #[allow(clippy::needless_borrow)]
        dependency: (&*ctx.dependency).clone(),
        #[allow(clippy::needless_borrow)]
        specifier: (&*ctx.specifier).to_owned(),
        pipeline: ctx.pipeline.clone(),
        plugin_options: state.rpc_plugin_options.clone(),
      })
      .await
  }
}
