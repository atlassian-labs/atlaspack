use std::collections::HashSet;

use swc_core::common::DUMMY_SP;
use swc_core::common::Mark;
use swc_core::common::SyntaxContext;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::utils::*;

/// Transformer that replaces global browser object references with `globalThis`.
///
/// This transform enables SSR compatibility by normalizing references to browser
/// global objects (eg. `window`, `document`, `navigator`, `global`) to use `globalThis`.
/// This allows code to work in both browser and server environments when combined
/// with appropriate polyfills or runtime checks.
///
/// # Example
///
/// Input:
/// ```js
/// const win = window;
/// const doc = document;
/// ```
///
/// Output:
/// ```js
/// const win = globalThis;
/// const doc = globalThis;
/// ```
///
/// # Scope Awareness
///
/// Only replaces unresolved (global) references. Local variables with the same
/// names are not affected:
///
/// ```js
/// function test() {
///   const window = {}; // Not replaced
///   return window;
/// }
/// const w = window; // Replaced with globalThis
/// ```
pub struct GlobalThisAliaser {
  /// Set of global identifiers to replace with `globalThis`
  pub aliases: HashSet<JsWord>,
  /// Mark used to identify unresolved (global) references
  pub unresolved_mark: Mark,
}

impl Default for GlobalThisAliaser {
  fn default() -> Self {
    Self {
      aliases: HashSet::from([
        "document".into(),
        "global".into(),
        "navigator".into(),
        "window".into(),
      ]),
      unresolved_mark: Mark::root(),
    }
  }
}

impl GlobalThisAliaser {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      ..Default::default()
    }
  }

  pub fn with_aliases(unresolved_mark: Mark, aliases: HashSet<JsWord>) -> Self {
    Self {
      aliases,
      unresolved_mark,
    }
  }

  pub fn should_transform(filename: &str) -> bool {
    !filename.contains("js.cookie.js")
  }
}

impl VisitMut for GlobalThisAliaser {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    if let Expr::Ident(ident) = node
      && is_unresolved(ident, self.unresolved_mark)
      && self.aliases.contains(&ident.sym)
    {
      *node = Expr::Ident(Ident::new(
        "globalThis".into(),
        DUMMY_SP,
        SyntaxContext::empty().apply_mark(self.unresolved_mark),
      ));
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
  fn test_global_this_aliasing() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const doc = document;
        const win = window;
        const nav = navigator;
        const glob = global;
        const other = someOther;
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const doc = globalThis;
        const win = globalThis;
        const nav = globalThis;
        const glob = globalThis;
        const other = someOther;
      "#}
    );
  }

  #[test]
  fn test_scoped_variables_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        function test() {
          const window = {};
          return window.location;
        }
        const global = window;
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function test() {
            const window = {};
            return window.location;
        }
        const global = globalThis;
      "#}
    );
  }

  #[test]
  fn test_excluded_files_not_processed() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const doc = document;
        const win = window;
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const doc = document;
        const win = window;
      "#}
    );
  }

  #[test]
  fn test_custom_aliases() {
    let mut custom_aliases = HashSet::new();
    custom_aliases.insert("custom".into());
    custom_aliases.insert("test".into());

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const customVar = custom;
        const testVar = test;
        const windowVar = window;
      "#},
      |run_test_context: RunTestContext| {
        GlobalThisAliaser::with_aliases(run_test_context.unresolved_mark, custom_aliases)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const customVar = globalThis;
        const testVar = globalThis;
        const windowVar = window;
      "#}
    );
  }

  #[test]
  fn test_member_expressions_preserved() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const location = window.location;
        const title = document.title;
        window.alert('test');
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const location = globalThis.location;
        const title = globalThis.title;
        globalThis.alert('test');
      "#}
    );
  }

  #[test]
  fn test_window_href_and_template_literals() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        window;
        window.location.href;
        window.blah();
        x(`${window.location.protocol}//${window.location.host}/browse/${issueKey}`);
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        globalThis;
        globalThis.location.href;
        globalThis.blah();
        x(`${globalThis.location.protocol}//${globalThis.location.host}/browse/${issueKey}`);
      "#}
    );
  }

  #[test]
  fn test_declared_window_variable_type() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        export const url = window.location.protocol + '/' + window.location.host;
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        export const url = globalThis.location.protocol + '/' + globalThis.location.host;
      "#}
    );
  }

  #[test]
  fn test_non_reference_window_identifiers_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        globalThis.window.location.origin;
        globalThis.window?.location.origin;
        globalThis?.window.location.origin;
        globalThis?.window?.location.origin;
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        globalThis.window.location.origin;
        globalThis.window?.location.origin;
        globalThis?.window.location.origin;
        globalThis?.window?.location.origin;
      "#}
    );
  }

  #[test]
  fn test_assignments_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        {
          const document = 25;
          const global = 'global value';
          const navigator = { key: 'value' };
          const window = true;
        }
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      "{\n    const document = 25;\n    const global = 'global value';\n    const navigator = {\n        key: 'value'\n    };\n    const window = true;\n}"
    );
  }

  #[test]
  fn test_jsx_attributes_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        React.createElement(AkRenderer, {
          document: 25,
          global: 'global value',
          navigator: { key: 'value' },
          window: true,
          documentLike: document,
          globalLike: global,
          navigatorLike: navigator,
          windowLike: window
        });
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        React.createElement(AkRenderer, {
            document: 25,
            global: 'global value',
            navigator: {
                key: 'value'
            },
            window: true,
            documentLike: globalThis,
            globalLike: globalThis,
            navigatorLike: globalThis,
            windowLike: globalThis
        });
      "#}
    );
  }

  #[test]
  fn test_object_keys_not_replaced() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        {
          const obj = {
            document,
            global,
            navigator,
            window,
            document: 25,
            global: 'global value',
            navigator: { key: 'value' },
            window: true,
            documentLike: document,
            globalLike: global,
            navigatorLike: navigator,
            windowLike: window
          }
        }
      "#},
      |run_test_context: RunTestContext| GlobalThisAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      "{\n    const obj = {\n        document,\n        global,\n        navigator,\n        window,\n        document: 25,\n        global: 'global value',\n        navigator: {\n            key: 'value'\n        },\n        window: true,\n        documentLike: globalThis,\n        globalLike: globalThis,\n        navigatorLike: globalThis,\n        windowLike: globalThis\n    };\n}"
    );
  }
}
