use std::collections::HashSet;
use std::sync::Arc;

use atlaspack_config::atlaspack_rc_config_loader::AtlaspackRcConfigLoader;
use atlaspack_config::atlaspack_rc_config_loader::LoadConfigOptions;
use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::PluginLogger;
use atlaspack_core::plugin::PluginOptions;
use tokio::sync::RwLock;

use crate::actions::asset_graph::AssetGraphAction;
use crate::actions::Action;
use crate::actions::ActionQueue;
use crate::actions::ActionType;
use crate::actions::Compilation;
use crate::atlaspack::AtlaspackState;
use crate::plugins::config_plugins::ConfigPlugins;

#[derive(Default)]
pub struct BuildOptions {}

pub async fn build(
  _build_options: BuildOptions,
  options: AtlaspackState,
) -> anyhow::Result<AssetGraph> {
  let c = Arc::new(Compilation {
    fs: options.fs.clone(),
    package_manager: options.package_manager.clone(),
    project_root: options.project_root.clone(),
    log_level: options.log_level.clone(),
    mode: options.mode.clone(),
    rpc: options.rpc.clone(),
    config_loader: options.config.clone(),
    plugins: options.plugins.clone(),
    env: options.env.unwrap_or_default().clone(),
    entries: options.entries.clone(),
    default_target_options: options.default_target_options.clone(),
    asset_graph: Arc::new(RwLock::new(AssetGraph::new())),
    asset_request_to_asset: Default::default(),
    pending_dependency_links: Default::default(),
  });

  let completed = HashSet::<u64>::new();
  let mut jobs = tokio::task::JoinSet::<anyhow::Result<u64>>::new();
  let (q, mut rx) = ActionQueue::new();

  q.next(ActionType::AssetGraph(AssetGraphAction {}))?;

  loop {
    // Try to get the next item from the queue
    let Ok((action, id)) = rx.try_recv() else {
      // If no jobs are running then exit
      if jobs.is_empty() {
        break;
      }
      // Wait for the next job to complete
      let Some(res) = jobs.join_next().await else {
        anyhow::bail!("terminated early")
      };
      res??;
      continue;
    };

    println!("{}", action);
    // dbg!(&action);
    // println!("");

    if completed.contains(&id) {
      continue;
    }

    jobs.spawn({
      let q = q.clone();
      let c = c.clone();
      async move {
        action.run(q, &c).await?;
        Ok(id)
      }
    });
  }

  anyhow::bail!("build complete")
}
