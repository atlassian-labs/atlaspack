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
use crate::plugins::config_plugins::ConfigPlugins;
use crate::state::State;
use crate::AtlaspackOptions;

pub struct BuildOptions {}

pub async fn build(
  _build_options: BuildOptions,
  options: AtlaspackOptions,
) -> anyhow::Result<AssetGraph> {
  let State {
    fs,
    package_manager,
    project_root,
    rpc,
    log_level,
    mode,
    core_path,
    entries,
    env,
    default_target_options,
  } = State::from_options(&options)?;

  let options = Arc::new(options);

  let rpc_worker = rpc.start()?;

  let (config, _files) =
    AtlaspackRcConfigLoader::new(Arc::clone(&fs), Arc::clone(&package_manager)).load(
      &project_root,
      LoadConfigOptions {
        additional_reporters: vec![], // TODO
        config: options.config.as_deref(),
        fallback_config: options.fallback_config.as_deref(),
      },
    )?;

  let config_loader = Arc::new(ConfigLoader {
    fs: Arc::clone(&fs),
    project_root: project_root.clone(),
    search_path: project_root.join("index"),
  });

  let plugins = Arc::new(ConfigPlugins::new(
    rpc_worker.clone(),
    config,
    PluginContext {
      config: Arc::clone(&config_loader),
      file_system: fs.clone(),
      options: Arc::new(PluginOptions {
        core_path: core_path.clone(),
        env: options.env.clone(),
        log_level: log_level.clone(),
        mode: mode.clone(),
        project_root: project_root.clone(),
      }),
      // TODO Initialise actual logger
      logger: PluginLogger::default(),
    },
  )?);

  let c = Arc::new(Compilation {
    options: options.clone(),
    fs: fs.clone(),
    package_manager: package_manager.clone(),
    project_root: project_root.clone(),
    log_level: log_level.clone(),
    mode: mode.clone(),
    rpc: rpc.clone(),
    config_loader: config_loader.clone(),
    plugins: plugins.clone(),
    env: env.clone(),
    entries: entries.clone(),
    default_target_options,
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
