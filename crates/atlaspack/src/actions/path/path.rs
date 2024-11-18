use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::asset_graph::AssetGraph;
use atlaspack_core::asset_graph::DependencyNode;
use atlaspack_core::asset_graph::DependencyState;
use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::BuildProgressEvent;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::Resolution;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::ResolvedResolution;
use atlaspack_core::plugin::ResolvingEvent;
use atlaspack_core::types::Dependency;
use atlaspack_resolver::parse_scheme;
use petgraph::graph::NodeIndex;

use super::super::ActionQueue;
use super::super::ActionType;
use super::super::Compilation;
use crate::actions::asset::AssetAction;
use crate::actions::Action;
use crate::plugins::PluginsRef;

#[derive(Hash, Debug)]
pub struct PathAction {
  pub dependency: Arc<Dependency>,
  pub node_index: NodeIndex,
  pub request_id: u64,
}

// TODO tracing, dev deps
impl Action for PathAction {
  async fn run(
    self,
    q: ActionQueue,
    Compilation {
      asset_graph,
      plugins,
      project_root,
      asset_request_to_asset,
      pending_dependency_links,
      ..
    }: &Compilation,
  ) -> anyhow::Result<()> {
    let result = self.exec(plugins).await?;

    let mut asset_graph = asset_graph.write().await;
    let asset_request_to_asset = asset_request_to_asset.write().await;

    let index = asset_graph
      .dependency_index(self.node_index.into())
      .unwrap();

    let DependencyNode {
      dependency,
      requested_symbols,
      state,
    } = &mut asset_graph.dependencies[index];

    if let PathRequestOutput::Excluded = result {
      *state = DependencyState::Excluded;
      return Ok(());
    };

    let PathRequestOutput::Resolved {
      path,
      code,
      pipeline,
      side_effects,
      query,
      can_defer,
    } = result
    else {
      anyhow::bail!("Probably should have used a match")
    };

    if !side_effects && can_defer && requested_symbols.is_empty() && !dependency.symbols.is_none() {
      *state = DependencyState::Deferred;
      return Ok(());
    }

    *state = DependencyState::Resolved;

    let asset_request = AssetAction {
      code,
      env: dependency.env.clone(),
      file_path: path,
      project_root: project_root.clone(),
      pipeline: pipeline,
      query: query,
      side_effects: side_effects,
    };
    let id = asset_request.id();

    if let Some(asset_node_index) = asset_request_to_asset.get(&id) {
      println!("Asset exists");
      // We have already completed this AssetRequest so we can connect the
      // Dependency to the Asset immediately
      asset_graph.add_edge(&self.node_index, asset_node_index);
      propagate_requested_symbols(
        &mut asset_graph,
        asset_node_index,
        &self.node_index,
        self.request_id.clone(),
        &q,
      );
      Ok(())
    } else {
      pending_dependency_links
        .write()
        .await
        .entry(id)
        .or_default()
        .insert(self.node_index.into());

      q.next(ActionType::Asset(asset_request))?;

      Ok(())
    }

    // The AssetRequest has already been kicked off but is yet to
    // complete. Register this Dependency to be connected once it
    // completes
    // self
    //   .waiting_asset_requests
    //   .entry(id)
    //   .and_modify(|nodes| {
    //     nodes.insert(node);
    //   })
    // .or_insert_with(|| HashSet::from([node]));
  }
}

impl PathAction {
  async fn exec(&self, plugins: &PluginsRef) -> anyhow::Result<PathRequestOutput> {
    plugins
      .reporter()
      .report(&ReporterEvent::BuildProgress(
        BuildProgressEvent::Resolving(ResolvingEvent {
          dependency: Arc::clone(&self.dependency),
        }),
      ))
      .await
      .expect("TODO this should be handled?");

    let (parsed_pipeline, specifier) = parse_scheme(&self.dependency.specifier)
      .and_then(|(pipeline, specifier)| {
        if plugins
          .named_pipelines()
          .contains(&String::from(pipeline.as_ref()))
        {
          Ok((Some(pipeline.to_string()), specifier))
        } else {
          Err(())
        }
      })
      .unwrap_or((None, self.dependency.specifier.as_ref()));

    let mut invalidations = Vec::new();

    for resolver in plugins.resolvers()?.iter() {
      let result = resolver
        .resolve(ResolveContext {
          dependency: Arc::clone(&self.dependency),
          pipeline: parsed_pipeline.clone(),
          specifier: String::from(specifier),
        })
        .await;

      let resolved = match result {
        Ok(result) => result,
        Err(error) => {
          // TODO: Forward the resolver diagnostic to the result
          tracing::info!("Resolver {:?} failed.\n{}", resolver, error);
          // Resolve failed so track the error and try the next resolver
          continue;
        }
      };

      invalidations.extend(resolved.invalidations);

      match resolved.resolution {
        Resolution::Unresolved => continue,
        Resolution::Excluded => return Ok(PathRequestOutput::Excluded),
        Resolution::Resolved(ResolvedResolution {
          can_defer,
          code,
          file_path,
          meta: _meta,
          pipeline,
          priority: _priority,
          query,
          side_effects,
        }) => {
          if !file_path.is_absolute() {
            return Err(diagnostic_error!(
              "{:?} must return an absolute path, but got {}",
              resolver,
              file_path.display()
            ));
          }

          // TODO resolution.diagnostics
          // TODO Set dependency meta and priority

          // return Ok(ResultAndInvalidations {
          //   invalidations,
          //   result: RequestResult::Path(PathRequestOutput::Resolved {
          //     can_defer,
          //     code,
          //     path: file_path,
          //     pipeline: pipeline
          //       .or(parsed_pipeline)
          //       .or(self.dependency.pipeline.clone()),
          //     query,
          //     side_effects,
          //   }),
          // });
          return Ok(PathRequestOutput::Resolved {
            can_defer,
            code,
            path: file_path,
            pipeline: pipeline
              .or(parsed_pipeline)
              .or(self.dependency.pipeline.clone()),
            query,
            side_effects,
          });
        }
      };
    }

    if self.dependency.is_optional {
      // todo!()
      // return Ok(ResultAndInvalidations {
      //   invalidations,
      //   result: RequestResult::Path(PathRequestOutput::Excluded),
      // });
      return Ok(PathRequestOutput::Excluded);
    }

    let resolve_from = self
      .dependency
      .resolve_from
      .as_ref()
      .or(self.dependency.source_path.as_ref());

    match resolve_from {
      None => Err(diagnostic_error!(
        "Failed to resolve {}",
        self.dependency.specifier
      )),
      Some(from) => Err(diagnostic_error!(
        "Failed to resolve {} from {}",
        self.dependency.specifier,
        from.display()
      )),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathRequestOutput {
  Excluded,
  Resolved {
    can_defer: bool,
    path: PathBuf,
    code: Option<String>,
    pipeline: Option<String>,
    query: Option<String>,
    side_effects: bool,
  },
}

fn propagate_requested_symbols(
  asset_graph: &mut AssetGraph,
  asset_node_index: &NodeIndex,
  incoming_dep_node_index: &NodeIndex,
  parent_request_id: u64,
  q: &ActionQueue,
) {
  asset_graph.propagate_requested_symbols(
    *asset_node_index,
    *incoming_dep_node_index,
    &mut |dependency_node_index: NodeIndex, dependency: Arc<Dependency>| {
      tracing::debug!(
        "queueing a path request from on_undeferred, {}",
        dependency.specifier
      );
      q.next(ActionType::Path(PathAction {
        dependency,
        node_index: dependency_node_index,
        request_id: parent_request_id,
      }))
      .unwrap();
    },
  );
}
