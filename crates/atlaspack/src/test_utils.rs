use std::path::{Path, PathBuf};
use std::sync::Arc;

use atlaspack_config::atlaspack_config_fixtures::default_config;
use atlaspack_core::types::{FeatureFlagValue, FeatureFlags};
use atlaspack_core::{
  config_loader::ConfigLoader,
  plugin::{PluginContext, PluginLogger, PluginOptions},
  types::AtlaspackOptions,
};
use atlaspack_filesystem::{in_memory_file_system::InMemoryFileSystem, FileSystemRef};
use atlaspack_plugin_rpc::rust::RustWorkerFactory;
use atlaspack_plugin_rpc::testing::TestingRpcFactory;
use atlaspack_plugin_rpc::RpcFactory;

use crate::{
  plugins::{config_plugins::ConfigPlugins, PluginsRef},
  request_tracker::RequestTracker,
};
use crate::{Atlaspack, AtlaspackInitOptions};

pub(crate) fn make_test_plugin_context() -> PluginContext {
  let fs = Arc::new(InMemoryFileSystem::default());

  fs.write_file(Path::new("package.json"), String::from("{}"));

  PluginContext {
    config: Arc::new(ConfigLoader::new(
      fs.clone(),
      PathBuf::default(),
      PathBuf::default(),
    )),
    file_system: fs.clone(),
    options: Arc::new(PluginOptions::default()),
    logger: PluginLogger::default(),
  }
}

pub(crate) fn config_plugins(ctx: PluginContext) -> PluginsRef {
  let fixture = default_config(Arc::new(PathBuf::default()));
  let rpc_factory = TestingRpcFactory::default();
  let rpc_worker = rpc_factory.start().unwrap();
  Arc::new(ConfigPlugins::new(rpc_worker, fixture.atlaspack_config, ctx).unwrap())
}

pub struct RequestTrackerTestOptions {
  pub fs: FileSystemRef,
  pub plugins: Option<PluginsRef>,
  pub project_root: PathBuf,
  pub search_path: PathBuf,
  pub atlaspack_options: AtlaspackOptions,
}

impl Default for RequestTrackerTestOptions {
  fn default() -> Self {
    Self {
      fs: Arc::new(InMemoryFileSystem::default()),
      plugins: None,
      project_root: PathBuf::default(),
      search_path: PathBuf::default(),
      atlaspack_options: AtlaspackOptions::default(),
    }
  }
}

pub(crate) fn request_tracker(options: RequestTrackerTestOptions) -> RequestTracker {
  let RequestTrackerTestOptions {
    fs,
    plugins,
    project_root,
    search_path,
    atlaspack_options,
  } = options;

  let config_loader = Arc::new(ConfigLoader::new(
    fs.clone(),
    project_root.clone(),
    search_path,
  ));

  let plugins = plugins.unwrap_or_else(|| {
    config_plugins(PluginContext {
      config: Arc::clone(&config_loader),
      file_system: fs.clone(),
      options: Arc::new(PluginOptions {
        core_path: atlaspack_options.core_path.clone(),
        env: atlaspack_options.env.clone(),
        log_level: atlaspack_options.log_level.clone(),
        mode: atlaspack_options.mode.clone(),
        project_root: project_root.clone(),
        feature_flags: Default::default(),
      }),
      logger: PluginLogger::default(),
    })
  });

  RequestTracker::new(
    Arc::clone(&config_loader),
    fs,
    Arc::new(atlaspack_options),
    plugins,
    project_root,
  )
}

pub async fn make_test_atlaspack(entries: &[impl AsRef<Path>]) -> anyhow::Result<Atlaspack> {
  let atlaspack = Atlaspack::new(AtlaspackInitOptions {
    db: create_db().unwrap(),
    fs: Some(Arc::new(atlaspack_resolver::OsFileSystem)),
    options: AtlaspackOptions {
      entries: entries
        .iter()
        .map(|e| e.as_ref().to_string_lossy().to_string())
        .collect(),
      core_path: get_core_path(),
      feature_flags: Arc::new(FeatureFlags(
        [("native_everything", FeatureFlagValue::Bool(true))]
          .iter()
          .map(|(k, v)| (k.to_string(), v.clone()))
          .collect(),
      )),
      ..Default::default()
    },
    package_manager: None,
    rpc: Arc::new(RustWorkerFactory::new().await?),
  })?;

  Ok(atlaspack)
}

pub fn setup_test_directory(name: &str) -> anyhow::Result<PathBuf> {
  let temp_dir = std::env::temp_dir().join("atlaspack").join(name);
  let _ = std::fs::remove_dir_all(&temp_dir);
  std::fs::create_dir_all(&temp_dir)?;
  let temp_dir = std::fs::canonicalize(&temp_dir)?;
  std::fs::write(temp_dir.join("yarn.lock"), r#"{}"#)?;
  std::fs::write(
    temp_dir.join(".parcelrc"),
    r#"{"extends": "@atlaspack/config-default"}"#,
  )?;
  symlink_core(&temp_dir)?;
  Ok(temp_dir)
}

pub fn symlink_core(temp_dir: &Path) -> anyhow::Result<()> {
  let repo_path = get_repo_path();

  let do_link = |source, target| -> anyhow::Result<()> {
    let source_path = repo_path.join(source);
    let target_path = temp_dir.join(target);

    tracing::debug!(?source_path, ?target_path, "Linking");

    std::fs::create_dir_all(target_path.parent().unwrap())?;
    std::os::unix::fs::symlink(&source_path, &target_path)?;

    Ok(())
  };

  do_link(
    "packages/configs/default",
    "node_modules/@atlaspack/config-default",
  )?;

  Ok(())
}

pub fn get_repo_path() -> PathBuf {
  let result = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
  std::fs::canonicalize(&result).unwrap()
}

pub fn get_core_path() -> PathBuf {
  let result = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/core");
  std::fs::canonicalize(&result).unwrap()
}

pub fn create_db() -> anyhow::Result<Arc<lmdb_js_lite::DatabaseHandle>> {
  let path = std::env::temp_dir()
    .join("atlaspack")
    .join("asset-graph-tests");
  let _ = std::fs::remove_dir_all(&path);

  let lmdb = lmdb_js_lite::get_database(lmdb_js_lite::LMDBOptions {
    path: path.to_string_lossy().to_string(),
    async_writes: false,
    map_size: Some((1024 * 1024 * 1024usize) as f64),
  })?;

  Ok(lmdb)
}
