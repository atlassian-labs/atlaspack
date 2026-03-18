use swc_core::common::{GLOBALS, Globals};

/// Run a closure with SWC globals initialized.
///
/// Required for tests that transitively call `private_ident!`
/// (e.g. through `hoist_sheet`).
pub fn with_globals<F: FnOnce()>(f: F) {
  GLOBALS.set(&Globals::new(), f);
}
