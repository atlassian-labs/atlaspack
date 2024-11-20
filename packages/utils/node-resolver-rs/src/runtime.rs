use once_cell::sync::OnceCell;
use std::future::Future;
use tokio::runtime::Builder;
use tokio::runtime::Handle;
use tokio::runtime::Runtime;

/// If there is no Tokio runtime, we spawn one for reuse.
/// This is useful in tests where no tokio runtime may be running
static RUNTIME_DEFAULT: OnceCell<Runtime> = OnceCell::new();

/// Use existing Tokio runtime or create a new one
pub fn handle() -> Handle {
  match tokio::runtime::Handle::try_current() {
    // Use existing runtime
    Ok(handle) => handle,
    // Spawn new runtime
    Err(_) => RUNTIME_DEFAULT
      .get_or_init(|| Builder::new_multi_thread().enable_all().build().unwrap())
      .handle()
      .to_owned(),
  }
}

pub fn block_on<R: Send + Sync, F: Future<Output = anyhow::Result<R>>>(
  fut: F,
) -> anyhow::Result<R> {
  let handle = handle();
  let _ = handle.enter();
  tokio::task::block_in_place(|| handle.block_on(fut))
}
