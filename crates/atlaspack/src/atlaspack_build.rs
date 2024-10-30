use atlaspack_core::asset_graph::AssetGraph;

use crate::AtlaspackOptions;

pub struct BuildOptions {}

pub async fn build(
  options: BuildOptions,
  global_options: AtlaspackOptions,
) -> anyhow::Result<AssetGraph> {
  anyhow::bail!("")
}

/*

self.runtime.block_on(async move {
  let AtlaspackState { config, plugins } = self.state().unwrap();

  let mut request_tracker = RequestTracker::new(
    config.clone(),
    self.fs.clone(),
    Arc::new(self.options.clone()),
    plugins.clone(),
    self.project_root.clone(),
  );

  let request_result = request_tracker.run_request(AssetGraphRequest {}).await?;

  let asset_graph = match request_result {
    RequestResult::AssetGraph(result) => result.graph,
    _ => panic!("TODO"),
  };

  Ok(asset_graph)
})
*/
