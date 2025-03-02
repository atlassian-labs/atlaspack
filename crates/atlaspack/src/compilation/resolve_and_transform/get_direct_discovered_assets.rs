use std::collections::HashSet;

use atlaspack_core::types::AssetWithDependencies;
use atlaspack_core::types::Dependency;

/// Direct discovered assets are discovered assets that don't have any
/// dependencies importing them. This means they need to be attached to the
/// original asset directly otherwise they'll be left out of the graph entirely.
///
/// CSS module JS export files are a good example of this.
pub fn get_direct_discovered_assets<'a>(
  discovered_assets: &'a [AssetWithDependencies],
  dependencies: &'a [Dependency],
) -> Vec<&'a AssetWithDependencies> {
  // Find all the discovered_asset unique keys
  let discovered_asset_unique_keys: HashSet<String> = discovered_assets
    .iter()
    .filter_map(|discovered_asset| discovered_asset.asset.unique_key.clone())
    .collect();

  let all_dependencies = dependencies.iter().chain(
    discovered_assets
      .iter()
      .flat_map(|discovered_asset| discovered_asset.dependencies.iter()),
  );

  // Find all the "indirect" discovered assets.
  // Assets that are pointed to by one of the generated dependencies within the
  // asset request
  let mut indirect_discovered_assets = HashSet::new();
  for dependency in all_dependencies {
    if discovered_asset_unique_keys.contains(&dependency.specifier) {
      indirect_discovered_assets.insert(dependency.specifier.clone());
    }
  }

  discovered_assets
    .iter()
    .filter(|discovered_asset| {
      !discovered_asset
        .asset
        .unique_key
        .as_ref()
        .is_some_and(|unique_key| indirect_discovered_assets.contains(unique_key))
    })
    .collect()
}
