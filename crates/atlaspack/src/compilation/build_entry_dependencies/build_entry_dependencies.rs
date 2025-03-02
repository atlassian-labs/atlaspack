use std::sync::Arc;

use atlaspack_core::types::Dependency;
use atlaspack_core::types::Target;
use pathdiff::diff_paths;

use super::resolve_package_targets::resolve_package_targets;
use crate::compilation::Compilation;

pub async fn build_entry_dependencies(
  Compilation {
    entries,
    config_loader,
    options,
    fs,
    project_root,
    asset_graph,
    entry_dependencies,
    ..
  }: &mut Compilation,
) -> anyhow::Result<()> {
  // TODO options.targets, should this still be supported?
  // TODO serve options
  for entry in entries {
    let package_targets = resolve_package_targets(entry, config_loader, options, fs)?;
    let targets = package_targets
      .into_iter()
      .flatten()
      .collect::<Vec<Target>>();

    for target in targets {
      let entry_path =
        diff_paths(&entry.file_path, &project_root).unwrap_or_else(|| entry.file_path.clone());

      let entry_path_str = entry_path.to_str().unwrap().to_string();

      let dependency = Dependency::entry(entry_path_str.clone(), target);
      let dep_node = asset_graph.add_entry_dependency(dependency.clone());
      entry_dependencies.push((dep_node, Arc::new(dependency)));
    }
  }

  entry_dependencies.sort_by_key(|(entry, _)| entry.clone());

  for (node_index, _) in entry_dependencies.iter() {
    asset_graph.add_edge(&asset_graph.root_node(), node_index);
  }

  Ok(())
}
