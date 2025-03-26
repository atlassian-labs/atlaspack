use std::sync::Arc;

use atlaspack::Atlaspack;
use atlaspack::AtlaspackInitOptions;
use parking_lot::Mutex;

type LazyAtlaspackResult = Result<Arc<Atlaspack>, anyhow::Error>;
type LazyAtlaspackInner = Arc<Mutex<Option<Arc<Atlaspack>>>>;

/// Initialize Atlaspack on first invocation.
/// Warning: cannot be done on main thread
#[derive(Clone)]
pub struct AtlaspackLazy(LazyAtlaspackInner, AtlaspackInitOptions);

impl AtlaspackLazy {
  pub fn new(options: AtlaspackInitOptions) -> Self {
    Self(Default::default(), options)
  }

  pub fn get(&self) -> anyhow::Result<Arc<Atlaspack>> {
    let mut atlaspack_container = self.0.lock();
    match atlaspack_container.as_ref() {
      Some(atlaspack) => Ok(Arc::clone(atlaspack)),
      None => {
        let atlaspack = match Atlaspack::new(self.1.clone()) {
          Ok(atlaspack) => Arc::new(atlaspack),
          Err(err) => return Err(anyhow::anyhow!("{:?}", err)),
        };
        atlaspack_container.replace(atlaspack.clone());
        Ok(atlaspack)
      }
    }
  }

  pub fn take(&self) -> anyhow::Result<Atlaspack> {
    let mut atlaspack_container = self.0.lock();
    match atlaspack_container.take() {
      Some(atlaspack) => Ok(Arc::into_inner(atlaspack).unwrap()),
      None => panic!("Naaw that ain't right"),
    }
  }
}
