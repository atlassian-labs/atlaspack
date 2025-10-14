use std::collections::HashSet;

use swc_core::common::DUMMY_SP;
use swc_core::common::Mark;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::utils::*;

/// Transformer that replaces `typeof` operations on browser APIs with the string `"undefined"`.
///
/// This transform enables SSR compatibility by replacing runtime `typeof` checks on
/// browser-only globals (eg. `window`, `document`, `navigator`) with the constant
/// string `"undefined"`. This allows bundlers to perform dead code elimination on
/// browser-specific code paths when building for server environments.
///
/// # Example
///
/// Input:
/// ```js
/// const windowType = typeof window;
/// if (typeof document !== 'undefined') {
///   document.body.style.color = 'red';
/// }
/// ```
///
/// Output:
/// ```js
/// const windowType = "undefined";
/// if ("undefined" !== 'undefined') {
///   document.body.style.color = 'red';
/// }
/// ```
///
/// # Scope Awareness
///
/// Only replaces unresolved (global) references. Local variables with the same
/// names are not affected:
///
/// ```js
/// function test() {
///   const window = {};
///   return typeof window; // Not replaced
/// }
/// const global = typeof window; // Replaced with "undefined"
/// ```
pub struct BrowserApiTypeofReplacer {
  /// Set of identifiers whose `typeof` operations should be replaced
  pub typeof_removals: HashSet<JsWord>,
  /// Mark used to identify unresolved (global) references
  pub unresolved_mark: Mark,
}

impl Default for BrowserApiTypeofReplacer {
  fn default() -> Self {
    Self {
      typeof_removals: HashSet::from(["window".into(), "document".into(), "navigator".into()]),
      unresolved_mark: Mark::root(),
    }
  }
}

impl BrowserApiTypeofReplacer {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      ..Default::default()
    }
  }

  #[allow(dead_code)]
  pub fn with_removals(unresolved_mark: Mark, removals: HashSet<JsWord>) -> Self {
    Self {
      typeof_removals: removals,
      unresolved_mark,
    }
  }

  pub fn should_transform(filename: &str) -> bool {
    !filename.contains("js.cookie.js")
  }
}

impl VisitMut for BrowserApiTypeofReplacer {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    if let Expr::Unary(unary) = node
      && unary.op == UnaryOp::TypeOf
      && let Expr::Ident(ident) = &*unary.arg
    {
      // Only replace if it's an unresolved reference (global) and in our removal list
      if is_unresolved(ident, self.unresolved_mark) && self.typeof_removals.contains(&ident.sym) {
        *node = Expr::Lit(Lit::Str(Str {
          value: "undefined".into(),
          span: DUMMY_SP,
          raw: None,
        }));
        return;
      }
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
  fn test_should_transform_excludes_js_cookie() {
    assert!(!BrowserApiTypeofReplacer::should_transform("js.cookie.js"));
    assert!(!BrowserApiTypeofReplacer::should_transform(
      "path/to/js.cookie.js"
    ));
    assert!(!BrowserApiTypeofReplacer::should_transform(
      "/absolute/path/js.cookie.js"
    ));
  }

  #[test]
  fn test_should_transform_allows_other_files() {
    assert!(BrowserApiTypeofReplacer::should_transform("index.js"));
    assert!(BrowserApiTypeofReplacer::should_transform("component.tsx"));
    assert!(BrowserApiTypeofReplacer::should_transform(
      "utils/helper.js"
    ));
    assert!(BrowserApiTypeofReplacer::should_transform("cookie.js"));
    assert!(BrowserApiTypeofReplacer::should_transform("js-cookie.js"));
  }

  #[test]
  fn test_browser_api_typeof_replacement() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const windowType = typeof window;
        const documentType = typeof document;
        const navigatorType = typeof navigator;
        const processType = typeof process;
      "#},
      |run_test_context: RunTestContext| {
        BrowserApiTypeofReplacer::new(run_test_context.unresolved_mark)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const windowType = "undefined";
        const documentType = "undefined";
        const navigatorType = "undefined";
        const processType = typeof process;
      "#}
    );
  }

  #[test]
  fn test_scoped_variables_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function test() {
          const window = {};
          return typeof window;
        }
        const global = typeof window;
      "#},
      |run_test_context: RunTestContext| {
        BrowserApiTypeofReplacer::new(run_test_context.unresolved_mark)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function test() {
            const window = {};
            return typeof window;
        }
        const global = "undefined";
      "#}
    );
  }

  #[test]
  fn test_custom_removals() {
    let mut custom_removals = HashSet::new();
    custom_removals.insert("custom".into());
    custom_removals.insert("test".into());

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const customType = typeof custom;
        const testType = typeof test;
        const windowType = typeof window;
      "#},
      |run_test_context: RunTestContext| {
        BrowserApiTypeofReplacer::with_removals(run_test_context.unresolved_mark, custom_removals)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const customType = "undefined";
        const testType = "undefined";
        const windowType = typeof window;
      "#}
    );
  }

  #[test]
  fn test_window_typeof_checks() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        typeof window === 'undefined';
        typeof window !== 'undefined';
      "#},
      |run_test_context: RunTestContext| {
        BrowserApiTypeofReplacer::new(run_test_context.unresolved_mark)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        "undefined" === 'undefined';
        "undefined" !== 'undefined';
      "#}
    );
  }
}
