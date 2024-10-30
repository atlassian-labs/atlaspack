use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::types::Environment;

use super::super::ActionQueue;
use super::super::ActionType;
use super::super::Compilation;
use crate::actions::asset::AssetAction;
use crate::actions::Action;

#[derive(Hash, Debug)]
pub struct LinkAction {
  pub project_root: PathBuf,
  pub code: Option<String>,
  pub env: Arc<Environment>,
  pub file_path: PathBuf,
  pub pipeline: Option<String>,
  pub query: Option<String>,
  pub side_effects: bool,
}

impl Action for LinkAction {
  async fn run(
    self,
    q: ActionQueue,
    Compilation { .. }: &Compilation,
  ) -> anyhow::Result<()> {
    let asset_request = AssetAction {
      code: self.code,
      env: self.env,
      file_path: self.file_path,
      project_root: self.project_root,
      pipeline: self.pipeline,
      query: self.query,
      side_effects: self.side_effects,
    };

    let id = asset_request.id();

    // if self.visited.insert(id) {
    //   self.request_id_to_dep_node_index.insert(id, node);
    //   self.work_count += 1;
    //   let _ = self
    //     .request_context
    //     .queue_request(asset_request, self.sender.clone());
    // } else if let Some(asset_node_index) = self.asset_request_to_asset.get(&id) {
    //   // We have already completed this AssetRequest so we can connect the
    //   // Dependency to the Asset immediately
    //   self.graph.add_edge(&node, asset_node_index);
    //   self.propagate_requested_symbols(*asset_node_index, node);
    // } else {
    //   // The AssetRequest has already been kicked off but is yet to
    //   // complete. Register this Dependency to be connected once it
    //   // completes
    //   self
    //     .waiting_asset_requests
    //     .entry(id)
    //     .and_modify(|nodes| {
    //       nodes.insert(node);
    //     })
    //     .or_insert_with(|| HashSet::from([node]));
    // }
    Ok(())
  }
}
