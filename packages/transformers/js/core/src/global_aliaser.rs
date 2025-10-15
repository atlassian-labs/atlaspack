use std::collections::HashMap;

use swc_core::common::DUMMY_SP;
use swc_core::common::Mark;
use swc_core::common::SyntaxContext;
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::utils::*;

#[derive(Debug, Clone)]
pub enum ReplacementValue {
  Identifier(String),
  Boolean(bool),
}

/// Transformer that replaces global identifiers with configured values.
///
/// This transform combines two related functionalities:
/// 1. Aliasing browser globals (eg. `window`, `document`) to `globalThis` for SSR compatibility
/// 2. Replacing SSR feature flags (eg. `__SERVER__`, `__SENTRY_DEBUG__`) with boolean literals
///
/// # Example
///
/// Input:
/// ```js
/// const win = window;
/// const doc = document;
/// const server = __SERVER__;
/// const debug = __SENTRY_DEBUG__;
/// ```
///
/// Output:
/// ```js
/// const win = globalThis;
/// const doc = globalThis;
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
///   const window = {}; // Not replaced
///   const __SERVER__ = 'local'; // Not replaced
///   return window;
/// }
/// const w = window; // Replaced with globalThis
/// const s = __SERVER__; // Replaced with true
/// ```
pub struct GlobalAliaser {
  /// Map of global identifiers to their replacement values
  pub mappings: HashMap<JsWord, ReplacementValue>,
  /// Mark used to identify unresolved (global) references
  pub unresolved_mark: Mark,
}

impl Default for GlobalAliaser {
  fn default() -> Self {
    Self {
      mappings: HashMap::from([
        (
          "document".into(),
          ReplacementValue::Identifier("globalThis".into()),
        ),
        (
          "global".into(),
          ReplacementValue::Identifier("globalThis".into()),
        ),
        (
          "navigator".into(),
          ReplacementValue::Identifier("globalThis".into()),
        ),
        (
          "window".into(),
          ReplacementValue::Identifier("globalThis".into()),
        ),
        ("__SERVER__".into(), ReplacementValue::Boolean(true)),
        ("__SENTRY_DEBUG__".into(), ReplacementValue::Boolean(false)),
        (
          "__SENTRY_TRACING__".into(),
          ReplacementValue::Boolean(false),
        ),
      ]),
      unresolved_mark: Mark::root(),
    }
  }
}

impl GlobalAliaser {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      ..Default::default()
    }
  }

  pub fn with_mappings(unresolved_mark: Mark, mappings: HashMap<JsWord, ReplacementValue>) -> Self {
    Self {
      mappings,
      unresolved_mark,
    }
  }

  pub fn with_config(unresolved_mark: Mark, config: &Option<HashMap<JsWord, JsWord>>) -> Self {
    let Some(config) = config else {
      return Self::new(unresolved_mark);
    };

    let mut mappings = HashMap::new();

    for (key, value) in config {
      if let Some(identifier) = value.strip_prefix("@") {
        mappings.insert(key.clone(), ReplacementValue::Identifier(identifier.into()));
      } else if value == "true" {
        mappings.insert(key.clone(), ReplacementValue::Boolean(true));
      } else if value == "false" {
        mappings.insert(key.clone(), ReplacementValue::Boolean(false));
      }
    }

    Self::with_mappings(unresolved_mark, mappings)
  }
}

impl VisitMut for GlobalAliaser {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    if let Expr::Ident(ident) = node
      && is_unresolved(ident, self.unresolved_mark)
      && let Some(replacement) = self.mappings.get(&ident.sym)
    {
      *node = match replacement {
        ReplacementValue::Identifier(identifier) => Expr::Ident(Ident::new(
          identifier.clone().into(),
          DUMMY_SP,
          SyntaxContext::empty().apply_mark(self.unresolved_mark),
        )),
        ReplacementValue::Boolean(value) => Expr::Lit(Lit::Bool((*value).into())),
      };
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
  fn test_ssr_global_mappings() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const server = __SERVER__;
        const debug = __SENTRY_DEBUG__;
        const tracing = __SENTRY_TRACING__;
        const other = __OTHER__;
      "#},
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
  fn test_combined_replacements() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const win = window;
        const doc = document;
        const server = __SERVER__;
        const debug = __SENTRY_DEBUG__;
        const other = someOther;
      "#},
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const win = globalThis;
        const doc = globalThis;
        const server = true;
        const debug = false;
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
          const __SERVER__ = 'local';
          return window.location;
        }
        const global = window;
        const server = __SERVER__;
      "#},
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        function test() {
            const window = {};
            const __SERVER__ = 'local';
            return window.location;
        }
        const global = globalThis;
        const server = true;
      "#}
    );
  }

  #[test]
  fn test_custom_mappings() {
    let mut custom_mappings = HashMap::new();
    custom_mappings.insert(
      "custom".into(),
      ReplacementValue::Identifier("globalThis".into()),
    );
    custom_mappings.insert(
      "test".into(),
      ReplacementValue::Identifier("replacement".into()),
    );
    custom_mappings.insert("__CUSTOM__".into(), ReplacementValue::Boolean(true));
    custom_mappings.insert("__TEST__".into(), ReplacementValue::Boolean(false));

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const customVar = custom;
        const testVar = test;
        const windowVar = window;
        const customFlag = __CUSTOM__;
        const testFlag = __TEST__;
      "#},
      |run_test_context: RunTestContext| {
        GlobalAliaser::with_mappings(run_test_context.unresolved_mark, custom_mappings)
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const customVar = globalThis;
        const testVar = replacement;
        const windowVar = window;
        const customFlag = true;
        const testFlag = false;
      "#}
    );
  }

  #[test]
  fn test_with_config() {
    let config = HashMap::from([
      ("custom".into(), "@globalThis".into()),
      ("test".into(), "@replacement".into()),
      ("__CUSTOM__".into(), "true".into()),
      ("__TEST__".into(), "false".into()),
    ]);

    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        const customVar = custom;
        const testVar = test;
        const windowVar = window;
        const customFlag = __CUSTOM__;
        const testFlag = __TEST__;
      "#},
      |run_test_context: RunTestContext| {
        GlobalAliaser::with_config(run_test_context.unresolved_mark, &Some(config))
      },
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        const customVar = globalThis;
        const testVar = replacement;
        const windowVar = window;
        const customFlag = true;
        const testFlag = false;
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      "{\n    const obj = {\n        document,\n        global,\n        navigator,\n        window,\n        document: 25,\n        global: 'global value',\n        navigator: {\n            key: 'value'\n        },\n        window: true,\n        documentLike: globalThis,\n        globalLike: globalThis,\n        navigatorLike: globalThis,\n        windowLike: globalThis\n    };\n}"
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
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
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

  #[test]
  fn test_mixed_usage_in_conditionals() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        if (__SERVER__ && window.location) {
          console.log(document.title);
        }
        const value = __SENTRY_DEBUG__ ? window : global;
      "#},
      |run_test_context: RunTestContext| GlobalAliaser::new(run_test_context.unresolved_mark),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        if (true && globalThis.location) {
            console.log(globalThis.title);
        }
        const value = false ? globalThis : globalThis;
      "#}
    );
  }
}
