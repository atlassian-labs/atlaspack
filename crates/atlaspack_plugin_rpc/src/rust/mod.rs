use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_config::PluginNode;
use atlaspack_core::plugin::BundlerPlugin;
use atlaspack_core::plugin::CompressorPlugin;
use atlaspack_core::plugin::NamerPlugin;
use atlaspack_core::plugin::OptimizerPlugin;
use atlaspack_core::plugin::PackagerPlugin;
use atlaspack_core::plugin::PluginContext;
use atlaspack_core::plugin::ReporterPlugin;
use atlaspack_core::plugin::Resolved;
use atlaspack_core::plugin::ResolverPlugin;
use atlaspack_core::plugin::RuntimePlugin;
use atlaspack_core::plugin::TransformerPlugin;
use atlaspack_core::types::Code;
use atlaspack_napi_helpers::js_callable::JsCallable;
use edon::napi::JsObject;
use edon::Nodejs;
use edon::NodejsWorker;
use napi::bindgen_prelude::FromNapiValue;
use napi::JsBuffer;
use napi::JsString;
use napi::JsUnknown;
use tracing::info;

use crate::javascript_plugin_api::JavaScriptPluginAPI;
use crate::javascript_plugin_api::LoadPluginOptions;
use crate::javascript_plugin_api::RpcAssetResult;
use crate::javascript_plugin_api::RpcTransformerOpts;
use crate::javascript_plugin_api::RunResolverResolve;
use crate::nodejs::RpcNodejsResolverPlugin;
use crate::nodejs::{
  NodejsRpcBundlerPlugin, NodejsRpcCompressorPlugin, NodejsRpcNamerPlugin,
  NodejsRpcOptimizerPlugin, NodejsRpcPackagerPlugin, NodejsRpcReporterPlugin,
  NodejsRpcRuntimePlugin, NodejsRpcTransformerPlugin,
};
use crate::RpcFactory;
use crate::RpcWorker;

pub struct RustWorkerFactory {
  node: Arc<Nodejs>,
}

impl RustWorkerFactory {
  pub async fn new() -> anyhow::Result<Self> {
    let libnode_path = get_libnode_path().await?;
    tracing::info!(?libnode_path, "Loading libnode");
    let node = Arc::new(edon::Nodejs::load_default(libnode_path)?);
    Ok(Self { node })
  }
}

impl RpcFactory for RustWorkerFactory {
  fn start(&self) -> anyhow::Result<Arc<dyn RpcWorker>> {
    Ok(Arc::new(RustWorker::new(self.node.clone())?))
  }
}

pub struct RustWorker {
  node: Arc<Nodejs>,
  js_workers: Arc<dyn JavaScriptPluginAPI + Send + Sync>,
}

impl RustWorker {
  pub fn new(node: Arc<Nodejs>) -> anyhow::Result<Self> {
    let js_workers: Arc<dyn JavaScriptPluginAPI + Send + Sync> =
      Arc::new(EdonWorkerPool::new(node.clone())?);

    Ok(Self { node, js_workers })
  }
}

impl RpcWorker for RustWorker {
  fn create_resolver(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> Result<Arc<dyn ResolverPlugin>, anyhow::Error> {
    Ok(Arc::new(RpcNodejsResolverPlugin::new(
      self.js_workers.clone(),
      ctx,
      plugin,
    )?))
  }

  fn create_transformer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Arc<dyn TransformerPlugin>> {
    Ok(Arc::new(NodejsRpcTransformerPlugin::new(
      self.js_workers.clone(),
      ctx,
      plugin,
    )?))
  }

  fn create_bundler(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn BundlerPlugin>> {
    Ok(Box::new(NodejsRpcBundlerPlugin::new(ctx, plugin)?))
  }

  fn create_compressor(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn CompressorPlugin>> {
    Ok(Box::new(NodejsRpcCompressorPlugin::new(ctx, plugin)?))
  }

  fn create_namer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn NamerPlugin>> {
    Ok(Box::new(NodejsRpcNamerPlugin::new(ctx, plugin)?))
  }

  fn create_optimizer(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn OptimizerPlugin>> {
    Ok(Box::new(NodejsRpcOptimizerPlugin::new(ctx, plugin)?))
  }

  fn create_packager(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn PackagerPlugin>> {
    Ok(Box::new(NodejsRpcPackagerPlugin::new(ctx, plugin)?))
  }

  fn create_reporter(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn ReporterPlugin>> {
    Ok(Box::new(NodejsRpcReporterPlugin::new(ctx, plugin)?))
  }

  fn create_runtime(
    &self,
    ctx: &PluginContext,
    plugin: &PluginNode,
  ) -> anyhow::Result<Box<dyn RuntimePlugin>> {
    Ok(Box::new(NodejsRpcRuntimePlugin::new(ctx, plugin)?))
  }
}

pub struct EdonWorkerPool {
  plugins: Vec<EdonJavaScriptPluginAPI>,
  current: AtomicUsize,
}

impl EdonWorkerPool {
  pub fn new(node: Arc<Nodejs>) -> anyhow::Result<Self> {
    let plugins = (0..num_cpus::get())
      .map(|i| {
        let worker = Arc::new(node.spawn_worker_thread()?);
        Ok(EdonJavaScriptPluginAPI::new(worker)?)
      })
      .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Self {
      plugins,
      current: AtomicUsize::new(0),
    })
  }
}

#[async_trait]
impl JavaScriptPluginAPI for EdonWorkerPool {
  async fn resolve(&self, opts: RunResolverResolve) -> anyhow::Result<Resolved> {
    let current = self.current.fetch_add(1, Ordering::Relaxed);
    self.plugins[current % self.plugins.len()]
      .resolve(opts)
      .await
  }

  async fn transform(
    &self,
    code: Code,
    source_map: Option<String>,
    run_transformer_opts: RpcTransformerOpts,
  ) -> anyhow::Result<(RpcAssetResult, Vec<u8>, Option<String>)> {
    let current = self.current.fetch_add(1, Ordering::Relaxed);
    self.plugins[current % self.plugins.len()]
      .transform(code, source_map, run_transformer_opts)
      .await
  }

  async fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    for plugin in &self.plugins {
      plugin.load_plugin(opts.clone()).await?;
    }
    Ok(())
  }
}

pub struct EdonJavaScriptPluginAPI {
  nodejs: Arc<NodejsWorker>,
  callbacks: EdonCallbacks,
}

struct EdonCallbacks {
  resolve_fn: JsCallable,
  transform_fn: JsCallable,
  load_plugin_fn: JsCallable,
}

impl EdonJavaScriptPluginAPI {
  pub fn new(node: Arc<NodejsWorker>) -> anyhow::Result<Self> {
    let (tx, rx) = std::sync::mpsc::channel();

    node.exec(move |env| {
      let run = || -> edon::napi::Result<EdonCallbacks> {
        let result: JsObject = env.run_script(
          r#"
const {AtlaspackWorker} = require('@atlaspack/core/lib/atlaspack-v3/worker/AtlaspackWorker.js');
const worker = new AtlaspackWorker();

const child_process = require('child_process');
const loggingExports = {};

for (const key of Object.keys(child_process)) {
  loggingExports[key] = (...args) => {
    try { throw new Error('e') } catch (e) {
      console.log('child_process', key, args, e.stack);
    }
    return child_process[key](...args);
  };
}

require.cache['child_process'] = {
  exports: loggingExports,
};

// const inspector = require('inspector');
// const session = new inspector.Session();
// session.connect();
// session.post(
//   'Profiler.enable',
//   () => session.post('Profiler.start', () => {}),
// );
// setTimeout(() => {
//   session.post('Profiler.stop', (sessionErr, data) => {
//     require('fs').writeFileSync(
//       'worker-cpu-profile-' + Date.now() + '.cpuprofile',
//       JSON.stringify(data.profile),
//       (writeErr) => {},
//     );
//   });
// }, 60000);

worker
        "#,
        )?;
        let bind = |method_name: &str| JsCallable::new_method_bound(method_name, &result);

        Ok(EdonCallbacks {
          resolve_fn: bind("runResolverResolve")?,
          transform_fn: bind("runTransformerTransform")?,
          load_plugin_fn: bind("loadPlugin")?,
        })
      };

      let result = run();
      tx.send(result.map_err(|err| anyhow::anyhow!("[edon] {}", err)));

      Ok(())
    });

    let callbacks = rx.recv()??;

    Ok(Self {
      nodejs: node,
      callbacks,
    })
  }
}

#[async_trait]
impl JavaScriptPluginAPI for EdonJavaScriptPluginAPI {
  async fn resolve(&self, opts: RunResolverResolve) -> anyhow::Result<Resolved> {
    self.callbacks.resolve_fn.call_serde(opts).await
  }

  async fn transform(
    &self,
    code: Code,
    source_map: Option<String>,
    run_transformer_opts: RpcTransformerOpts,
  ) -> anyhow::Result<(RpcAssetResult, Vec<u8>, Option<String>)> {
    let result = self
      .callbacks
      .transform_fn
      .call(
        move |env| {
          let run_transformer_opts = env.to_js_value(&run_transformer_opts)?;

          let mut contents = env.create_buffer(code.len())?;
          contents.copy_from_slice(&code);

          let map = if let Some(map) = source_map {
            env.create_string(&map)?.into_unknown()
          } else {
            env.get_undefined()?.into_unknown()
          };

          Ok(vec![run_transformer_opts, contents.into_unknown(), map])
        },
        |env, return_value| {
          let return_value = JsObject::from_unknown(return_value)?;

          let transform_result = return_value.get_element::<JsUnknown>(0)?;
          let transform_result = env.from_js_value::<RpcAssetResult, _>(transform_result)?;

          let contents = return_value.get_element::<JsBuffer>(1)?;
          let contents = contents.into_value()?.to_vec();

          let map = return_value.get_element::<JsString>(2)?.into_utf8()?;
          let map = if map.is_empty() {
            None
          } else {
            Some(map.into_owned()?)
          };

          Ok((transform_result, contents, map))
        },
      )
      .await?;

    Ok(result)
  }

  async fn load_plugin(&self, opts: LoadPluginOptions) -> anyhow::Result<()> {
    self.callbacks.load_plugin_fn.call_serde(opts).await
  }
}

async fn get_libnode_path() -> anyhow::Result<PathBuf> {
  let home_dir =
    home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
  let atlaspack_dir = home_dir.join(".atlaspack");
  let platform = std::env::consts::OS;

  let libnode_path = if platform == "linux" {
    atlaspack_dir.join("libnode.so")
  } else if platform == "macos" {
    atlaspack_dir.join("libnode.dylib")
  } else {
    return Err(anyhow::anyhow!("Unsupported platform: {}", platform));
  };

  if libnode_path.exists() {
    return Ok(libnode_path);
  }

  let arch = if std::env::consts::ARCH.contains("x86") {
    "amd64"
  } else {
    "arm64"
  };
  let node_version = "v22";
  let archive = format!("libnode-{platform}-{arch}.tar.gz");
  let download_url = format!(
    "https://github.com/alshdavid/libnode-prebuilt/releases/download/{node_version}/{archive}"
  );
  tracing::info!(?platform, ?arch, ?download_url, "Downloading libnode");
  let response = reqwest::get(&download_url).await?;

  std::fs::create_dir_all(&atlaspack_dir)?;

  let target_path = atlaspack_dir.join(archive);

  let mut file = std::fs::File::create(&target_path)?;
  let mut content = std::io::Cursor::new(response.bytes().await?);
  std::io::copy(&mut content, &mut file)?;

  // uncompress
  std::process::Command::new("tar")
    .arg("-xvf")
    .arg(&target_path)
    .current_dir(&atlaspack_dir)
    .output()?;

  Ok(libnode_path)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_edon() {
    let libnode_path = get_libnode_path().await.unwrap();
    let node = edon::Nodejs::load_default(libnode_path).unwrap();
    let worker = node.spawn_worker_thread().unwrap();
    worker.eval_blocking("require('@atlaspack/core');").unwrap();
  }
}
