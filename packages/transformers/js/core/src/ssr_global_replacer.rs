use std::collections::HashMap;

use swc_core::common::Mark;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::utils::*;

/// Transformer that replaces SSR global identifiers with boolean literals.
///
/// This transform enables compile-time feature flags for server-side rendering
/// by replacing special global identifiers (eg. `__SERVER__`, `__SENTRY_DEBUG__`)
/// with their corresponding boolean values. This allows dead code elimination
/// to remove unused code paths during bundling.
///
/// # Example
///
/// Input:
/// ```js
/// const server = __SERVER__;
/// const debug = __SENTRY_DEBUG__;
/// ```
///
/// Output:
/// ```js
/// const server = true;
/// const debug = false;
/// ```
///
/// # Scope Awareness
///
/// Only replaces unresolved (global) references. Local variables with the same
/// names are not affected:
///
/// ```js
/// function test() {
///   const __SERVER__ = 'local'; // Not replaced
///   return __SERVER__;
/// }
/// const global = __SERVER__; // Replaced with true
/// ```
pub struct SsrGlobalReplacer {
  /// Map of global identifiers to their boolean replacement values
  pub mappings: HashMap<JsWord, bool>,
  /// Mark used to identify unresolved (global) references
  pub unresolved_mark: Mark,
}

impl Default for SsrGlobalReplacer {
  fn default() -> Self {
    Self {
      mappings: HashMap::from([
        ("__SERVER__".into(), true),
        ("__SENTRY_DEBUG__".into(), false),
        ("__SENTRY_TRACING__".into(), false),
      ]),
      unresolved_mark: Mark::root(),
    }
  }
}

impl SsrGlobalReplacer {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      ..Default::default()
    }
  }

  #[allow(dead_code)]
  pub fn with_mappings(unresolved_mark: Mark, mappings: HashMap<JsWord, bool>) -> Self {
    Self {
      mappings,
      unresolved_mark,
    }
  }
}

impl VisitMut for SsrGlobalReplacer {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    if let Expr::Ident(ident) = node
      && is_unresolved(ident, self.unresolved_mark)
      && let Some(&value) = self.mappings.get(&ident.sym)
    {
      *node = Expr::Lit(Lit::Bool(value.into()));
      return;
    }

    node.visit_mut_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunTestContext, RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_ssr_global_mappings() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const server = __SERVER__;
        const debug = __SENTRY_DEBUG__;
        const tracing = __SENTRY_TRACING__;
        const other = __OTHER__;
      "#},
      |run_test_context: RunTestContext| SsrGlobalReplacer::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const server = true;
        const debug = false;
        const tracing = false;
        const other = __OTHER__;
      "#}
    );
  }

  #[test]
  fn test_custom_mappings() {
    let mut custom_mappings = HashMap::new();
    custom_mappings.insert("__CUSTOM__".into(), true);
    custom_mappings.insert("__TEST__".into(), false);

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const custom = __CUSTOM__;
        const test = __TEST__;
      "#},
      |run_test_context: RunTestContext| {
        SsrGlobalReplacer::with_mappings(run_test_context.unresolved_mark, custom_mappings)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const custom = true;
        const test = false;
      "#}
    );
  }

  #[test]
  fn test_scoped_variables_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function test() {
          const __SERVER__ = 'local';
          return __SERVER__;
        }
        const global = __SERVER__;
      "#},
      |run_test_context: RunTestContext| SsrGlobalReplacer::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function test() {
            const __SERVER__ = 'local';
            return __SERVER__;
        }
        const global = true;
      "#}
    );
  }

  #[test]
  fn test_server_variables_inline() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        if (__SERVER__) {
          serverIsTrue();
        } else {
          serverIsFalse();
        }
        if (typeof __SERVER__ !== 'undefined') {
          serverVarExists();
        } else {
          serverVarNotExists();
        }
      "#},
      |run_test_context: RunTestContext| SsrGlobalReplacer::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        if (true) {
            serverIsTrue();
        } else {
            serverIsFalse();
        }
        if (typeof true !== 'undefined') {
            serverVarExists();
        } else {
            serverVarNotExists();
        }
      "#}
    );
  }
}
