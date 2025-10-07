use std::collections::HashMap;

use swc_core::common::Mark;
use swc_core::ecma::ast::Expr;
use swc_core::ecma::ast::Lit;
use swc_core::ecma::ast::Str;
use swc_core::ecma::ast::UnaryOp;
use swc_core::ecma::atoms::Atom;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::utils::is_unresolved;

/// Replaces `typeof` unary operator expressions with configured replacement values.
///
/// # Example
/// ## Input
/// ```js
/// const x = typeof require;
/// const m = typeof module;
/// const e = typeof document;
/// ```
///
/// ## Output
/// ```js
/// const x = "function";
/// const m = "object";
/// const e = "undefined";
/// ```
pub struct TypeofReplacer {
  unresolved_mark: Mark,
  replacements: HashMap<Atom, Atom>,
}

impl Default for TypeofReplacer {
  fn default() -> Self {
    Self {
      unresolved_mark: Mark::new(),
      replacements: HashMap::from([
        ("require".into(), "function".into()),
        ("module".into(), "object".into()),
        ("exports".into(), "object".into()),
      ]),
    }
  }
}

impl TypeofReplacer {
  pub fn new(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      ..Default::default()
    }
  }

  pub fn new_ssr(unresolved_mark: Mark) -> Self {
    Self {
      unresolved_mark,
      replacements: HashMap::from([
        ("require".into(), "function".into()),
        ("module".into(), "object".into()),
        ("exports".into(), "object".into()),
        ("window".into(), "undefined".into()),
        ("document".into(), "undefined".into()),
        ("navigator".into(), "undefined".into()),
      ]),
    }
  }
}

impl TypeofReplacer {
  /// Given an expression, optionally return a replacement if it happens to be `typeof $symbol` for
  /// identifiers configured in the replacements map.
  fn get_replacement(&mut self, node: &Expr) -> Option<Expr> {
    let Expr::Unary(unary) = &node else {
      return None;
    };
    if unary.op != UnaryOp::TypeOf {
      return None;
    }

    let Expr::Ident(ident) = &*unary.arg else {
      return None;
    };

    // Check if this identifier is unresolved and has a replacement configured
    if !is_unresolved(ident, self.unresolved_mark) {
      return None;
    }

    let replacement_value = self.replacements.get(&ident.sym)?;

    Some(Expr::Lit(Lit::Str(Str {
      span: unary.span,
      value: replacement_value.clone(),
      raw: None,
    })))
  }
}

impl VisitMut for TypeofReplacer {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    let Some(replacement) = self.get_replacement(node) else {
      node.visit_mut_children_with(self);
      return;
    };

    *node = replacement;
  }
}

#[cfg(test)]
mod tests {
  use atlaspack_swc_runner::test_utils::{RunVisitResult, run_test_visit};
  use indoc::indoc;

  use super::*;

  #[test]
  fn test_default_replacements() {
    let code = indoc! {r#"
      const x = typeof require;
      const m = typeof module;
      const e = typeof exports;
      const w = typeof window;
      const d = typeof document;
      const n = typeof navigator;
      const p = typeof process;
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(code, |context| TypeofReplacer::new(context.unresolved_mark));

    assert_eq!(
      output_code,
      indoc! {r#"
        const x = "function";
        const m = "object";
        const e = "object";
        const w = typeof window;
        const d = typeof document;
        const n = typeof navigator;
        const p = typeof process;
      "#}
    );
  }

  #[test]
  fn test_ssr_replacements() {
    let code = indoc! {r#"
      const x = typeof require;
      const m = typeof module;
      const e = typeof exports;
      const w = typeof window;
      const d = typeof document;
      const n = typeof navigator;
      const p = typeof process;
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(code, |context| {
      TypeofReplacer::new_ssr(context.unresolved_mark)
    });

    assert_eq!(
      output_code,
      indoc! {r#"
        const x = "function";
        const m = "object";
        const e = "object";
        const w = "undefined";
        const d = "undefined";
        const n = "undefined";
        const p = typeof process;
      "#}
    );
  }

  #[test]
  fn test_typeof_in_expressions() {
    let code = indoc! {r#"
      const x = typeof require === 'function';
      typeof module === 'object';
      typeof module !== 'object';
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(code, |context| TypeofReplacer::new(context.unresolved_mark));

    assert_eq!(
      output_code,
      indoc! {r#"
        const x = "function" === 'function';
        "object" === 'object';
        "object" !== 'object';
      "#}
    );
  }

  #[test]
  fn test_respects_variable_scope() {
    let code = indoc! {r#"
      function wrapper({ require, exports }) {
        const x = typeof require;
        const e = typeof exports;
        const m = typeof module;
      }
      const global = typeof require;
    "#};

    let RunVisitResult { output_code, .. } =
      run_test_visit(code, |context| TypeofReplacer::new(context.unresolved_mark));

    assert_eq!(
      output_code,
      indoc! {r#"
        function wrapper({ require, exports }) {
            const x = typeof require;
            const e = typeof exports;
            const m = "object";
        }
        const global = "function";
      "#}
    );
  }

  #[test]
  fn test_custom_replacements() {
    let code = indoc! {r#"
      const x = typeof require;
      const m = typeof module;
      const c = typeof custom;
    "#};

    let custom_replacements = HashMap::from([
      ("require".into(), "string".into()),
      ("module".into(), "number".into()),
      ("custom".into(), "symbol".into()),
    ]);

    let RunVisitResult { output_code, .. } = run_test_visit(code, |context| TypeofReplacer {
      unresolved_mark: context.unresolved_mark,
      replacements: custom_replacements,
    });

    assert_eq!(
      output_code,
      indoc! {r#"
        const x = "string";
        const m = "number";
        const c = "symbol";
      "#}
    );
  }
}
