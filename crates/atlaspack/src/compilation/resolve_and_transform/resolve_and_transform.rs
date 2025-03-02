use std::sync::Arc;

use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::types::Dependency;
use atlaspack_resolver::parse_scheme;

use super::super::Compilation;
use super::resolve::resolve;
use crate::plugins::PluginsRef;

pub async fn resolve_and_transform(
  Compilation {
    entry_dependencies,
    plugins,
    ..
  }: &mut Compilation,
) -> anyhow::Result<()> {
  let mut queue = Vec::from_iter(entry_dependencies.drain(0..));

  while let Some((dependency_nx, dependency)) = queue.pop() {
    let plugins = Arc::clone(&plugins);
    let resolution = resolve(dependency, plugins).await?;
    //
  }

  Ok(())
}
