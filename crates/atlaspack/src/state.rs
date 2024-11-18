// use std::collections::BTreeMap;
// use std::path::PathBuf;
// use std::sync::Arc;

// use atlaspack_core::types::BuildMode;
// use atlaspack_core::types::DefaultTargetOptions;
// use atlaspack_core::types::LogLevel;
// use atlaspack_filesystem::FileSystemRef;
// use atlaspack_package_manager::NodePackageManager;
// use atlaspack_package_manager::PackageManager;
// use atlaspack_package_manager::PackageManagerRef;
// use atlaspack_plugin_rpc::RpcFactoryRef;
// use atlaspack_resolver::FileSystem;
// use atlaspack_resolver::OsFileSystem;

// use crate::project_root::infer_project_root;

// pub type EnvMap = BTreeMap<String, String>;

// pub struct State {
//   pub fs: FileSystemRef,
//   pub package_manager: PackageManagerRef,
//   pub project_root: PathBuf,
//   pub log_level: LogLevel,
//   pub mode: BuildMode,
//   pub rpc: RpcFactoryRef,
//   pub entries: Vec<String>,
//   pub core_path: PathBuf,
//   pub env: EnvMap,
//   pub default_target_options: DefaultTargetOptions,
// }

// impl State {
//   pub fn from_options(options: &AtlaspackOptions) -> anyhow::Result<Self> {
//     let Some(rpc) = &options.rpc else {
//       anyhow::bail!("Not running with connection to Nodejs")
//     };

//     let fs: Arc<dyn FileSystem> = match &options.fs {
//       Some(fs) => fs.clone(),
//       None => Arc::new(OsFileSystem::default()),
//     };

//     let entries = match &options.entries {
//       Some(entries) => entries.clone(),
//       None => Default::default(),
//     };

//     let core_path = match &options.core_path {
//       Some(core_path) => core_path.clone(),
//       None => Default::default(),
//     };

//     let project_root = infer_project_root(fs.clone(), entries.clone())?;

//     let package_manager: Arc<dyn PackageManager> = match &options.package_manager {
//       Some(package_manager) => package_manager.clone(),
//       None => Arc::new(NodePackageManager::new(project_root.clone(), fs.clone())),
//     };

//     let mode = match &options.mode {
//       Some(mode) => mode.clone(),
//       None => Default::default(),
//     };

//     let log_level = match &options.log_level {
//       Some(log_level) => log_level.clone(),
//       None => Default::default(),
//     };

//     let env = match &options.env {
//       Some(env) => env.clone(),
//       None => Default::default(),
//     };

//     let default_target_options = match &options.default_target_options {
//       Some(v) => v.clone(),
//       None => Default::default(),
//     };

//     Ok(Self {
//       fs,
//       project_root,
//       package_manager,
//       mode,
//       log_level,
//       rpc: rpc.clone(),
//       entries,
//       core_path,
//       env,
//       default_target_options,
//     })
//   }
// }
