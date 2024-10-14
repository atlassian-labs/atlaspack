use std::fmt;
use std::fmt::Debug;
use std::fs::File;

use atlaspack_config::PluginNode;
use atlaspack_core::plugin::CompressedFile;
use atlaspack_core::plugin::CompressorPlugin;
use atlaspack_core::plugin::PluginContext;

pub struct NodejsRpcCompressorPlugin {
  _name: String,
}

impl Debug for NodejsRpcCompressorPlugin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "RpcCompressorPlugin")
  }
}

impl NodejsRpcCompressorPlugin {
  pub fn new(_ctx: &PluginContext, plugin: &PluginNode) -> anyhow::Result<Self> {
    Ok(NodejsRpcCompressorPlugin {
      _name: plugin.package_name.clone(),
    })
  }
}

impl CompressorPlugin for NodejsRpcCompressorPlugin {
  fn compress(&self, _file: &File) -> Result<Option<CompressedFile>, String> {
    todo!()
  }
}
