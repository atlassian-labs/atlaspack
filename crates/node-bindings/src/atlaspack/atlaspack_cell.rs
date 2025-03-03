use std::sync::Arc;
use std::thread;

use atlaspack::Atlaspack;
use atlaspack::AtlaspackInitOptions;
use once_cell::sync::OnceCell;

/// Initialize Atlaspack in another thread and store the result.
/// the result is available on the first access of Atlaspack
#[derive(Clone)]
pub struct AtlaspackCell(Arc<OnceCell<anyhow::Result<Arc<Atlaspack>>>>);

impl AtlaspackCell {
  pub fn new(options: AtlaspackInitOptions) -> Self {
    let inner = Arc::new(OnceCell::<anyhow::Result<Arc<Atlaspack>>>::new());

    thread::spawn({
      let inner = inner.clone();
      move || {
        inner
          .set(match Atlaspack::new(options) {
            Ok(atlaspack) => Ok(Arc::new(atlaspack)),
            Err(err) => Err(err),
          })
          .ok()
      }
    });

    Self(inner)
  }

  pub fn get(&self) -> anyhow::Result<Arc<Atlaspack>> {
    match self.0.wait().as_ref() {
      Ok(atlaspack) => Ok(Arc::clone(atlaspack)),
      Err(error) => Err(anyhow::anyhow!("{:?}", error)),
    }
  }
}
