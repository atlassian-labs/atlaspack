use std::sync::Arc;

use atlaspack::Atlaspack;
use atlaspack::AtlaspackInitOptions;
use parking_lot::Mutex;

// type LazyAtlaspackResult = Result<Arc<Atlaspack>, anyhow::Error>;
type LazyAtlaspackInner = Arc<Mutex<Option<anyhow::Result<Arc<Atlaspack>>>>>;

/// Initialize Atlaspack on first invocation.
/// Warning: cannot be done on main thread
#[derive(Clone)]
pub struct AtlaspackLazy {
  atlaspack_container: LazyAtlaspackInner,
  options: AtlaspackInitOptions,
}

impl AtlaspackLazy {
  pub fn new(options: AtlaspackInitOptions) -> Self {
    Self {
      atlaspack_container: Default::default(),
      options,
    }
  }

  pub fn get(&self) -> anyhow::Result<Arc<Atlaspack>> {
    let mut atlaspack_container = self.atlaspack_container.lock();

    if atlaspack_container.is_none() {
      atlaspack_container.replace(match Atlaspack::new(self.options.clone()) {
        Ok(atlaspack) => Ok(Arc::new(atlaspack)),
        Err(err) => Err(err),
      });
    }

    match atlaspack_container.as_ref() {
      Some(Ok(atlaspack)) => Ok(Arc::clone(atlaspack)),
      Some(Err(error)) => Err(anyhow::anyhow!("{:?}", error)),
      // Should never happen
      None => Err(anyhow::anyhow!("Atlaspack cannot initialize")),
    }
  }

  pub fn take(&self) -> anyhow::Result<Atlaspack> {
    let mut atlaspack_container = self.atlaspack_container.lock();
    match atlaspack_container.take() {
      Some(Ok(atlaspack)) => match Arc::into_inner(atlaspack) {
        Some(atlaspack) => Ok(atlaspack),
        None => Err(anyhow::anyhow!(
          "Cannot take because there is more than one Atlaspack instance currently held"
        )),
      },
      Some(Err(error)) => Err(error),
      None => Err(anyhow::anyhow!("Atlaspack not initialized")),
    }
  }
}
