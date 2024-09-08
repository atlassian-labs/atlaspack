use swc_core::ecma::visit::{Fold, VisitMut};

use crate::runner::{run_fold, run_visit};
pub use crate::runner::{RunContext, RunVisitResult};

/// In the future this might be a different type to `RunContext`
pub type RunTestContext = RunContext;

/// Helper to test SWC visitors.
///
/// * Parse `code` with SWC
/// * Run a visitor over it
/// * Return the result
///
pub fn run_test_visit<V: VisitMut>(
  code: &str,
  make_visit: impl FnOnce(RunTestContext) -> V,
) -> RunVisitResult<V> {
  run_visit(code, make_visit).unwrap()
}

/// Same as `run_visit` but for `Fold` instances
#[allow(unused)]
pub fn run_test_fold<V: Fold>(
  code: &str,
  make_fold: impl FnOnce(RunTestContext) -> V,
) -> RunVisitResult<V> {
  run_fold(code, make_fold).unwrap()
}
