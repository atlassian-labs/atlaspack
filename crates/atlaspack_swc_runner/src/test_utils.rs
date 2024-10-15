use regex::Regex;
use swc_core::ecma::visit::{Fold, Visit, VisitMut};

use crate::runner::{run_fold, run_visit, run_visit_const};
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

pub fn run_test_visit_const<V: Visit>(
  code: &str,
  make_visit: impl FnOnce(RunTestContext) -> V,
) -> RunVisitResult<V> {
  run_visit_const(code, make_visit).unwrap()
}

/// Same as `run_visit` but for `Fold` instances
#[allow(unused)]
pub fn run_test_fold<V: Fold>(
  code: &str,
  make_fold: impl FnOnce(RunTestContext) -> V,
) -> RunVisitResult<V> {
  run_fold(code, make_fold).unwrap()
}

/// Remove whitespace from line starts and ends
#[allow(unused)]
pub fn remove_code_whitespace(code: &str) -> String {
  let re = Regex::new(r"\s*\n\s*").unwrap();
  re.replace_all(code, "\n").trim().to_string()
}
