use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::plugin::Resolution;
use crate::plugin::ResolveContext;
use crate::plugin::ResolverPlugin;
use crate::types::Dependency;

#[derive(Default)]
pub struct StaticResolver {
  resolvers: Arc<OnceLock<Vec<Arc<dyn ResolverPlugin>>>>,
}

impl StaticResolver {
  pub fn init(&self, resolvers: Vec<Arc<dyn ResolverPlugin>>) {
    self.resolvers.set(resolvers).unwrap();
  }

  pub fn resolve(&self, specifier: &str, resolve_from: &Path) -> anyhow::Result<Option<PathBuf>> {
    for resolver in self.resolvers.get().unwrap().iter() {
      let result = resolver.resolve(ResolveContext {
        dependency: Arc::new(Dependency {
          resolve_from: Some(resolve_from.to_path_buf()),
          specifier: specifier.to_string(),
          ..Default::default()
        }),
        pipeline: None,
        specifier: specifier.to_string(),
      })?;

      match result.resolution {
        Resolution::Resolved(resolution) => return Ok(Some(resolution.file_path)),
        _ => continue,
      }
    }

    Ok(None)
  }
}
