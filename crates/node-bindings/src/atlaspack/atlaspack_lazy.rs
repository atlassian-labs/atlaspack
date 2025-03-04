use std::sync::Arc;

use atlaspack::Atlaspack;
use atlaspack::AtlaspackInitOptions;
use once_cell::sync::Lazy;

type LazyAtlaspackResult = Result<Arc<Atlaspack>, anyhow::Error>;
type LazyAtlaspackInner =
  Arc<Lazy<LazyAtlaspackResult, Box<dyn FnOnce() -> LazyAtlaspackResult + Send + 'static>>>;

/// Initialize Atlaspack on first invocation.
/// Warning: cannot be done on main thread
#[derive(Clone)]
pub struct AtlaspackLazy(LazyAtlaspackInner);

impl AtlaspackLazy {
  pub fn new(options: AtlaspackInitOptions) -> Self {
    Self(Arc::new(Lazy::new(Box::new(
      move || match Atlaspack::new(options) {
        Ok(atlaspack) => Ok(Arc::new(atlaspack)),
        Err(err) => Err(err),
      },
    ))))
  }

  pub fn get(&self) -> anyhow::Result<Arc<Atlaspack>> {
    match &**self.0 {
      Ok(atlaspack) => Ok(Arc::clone(atlaspack)),
      Err(error) => Err(anyhow::anyhow!("{:?}", error)),
    }
  }
}
