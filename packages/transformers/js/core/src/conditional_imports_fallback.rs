use swc_core::atoms::{atom, Atom};
use swc_core::common::{Mark, Span, DUMMY_SP};
use swc_core::ecma::ast::{CallExpr, Callee, Expr, ExprOrSpread, Ident, Lit, Str};
use swc_core::ecma::visit::VisitMut;
use swc_core::quote;

use crate::utils::match_str;

pub struct ConditionalImportsFallback {
  pub unresolved_mark: Mark,
}

impl VisitMut for ConditionalImportsFallback {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    let Expr::Call(call) = node else {
      return;
    };

    let Callee::Expr(callee_expr) = &call.callee else {
      return;
    };

    let Expr::Ident(callee_ident) = &**callee_expr else {
      return;
    };

    if callee_ident.sym != atom!("importCond") && call.args.len() != 3 {
      // Not an importCond
      return;
    }

    if callee_ident.span.ctxt.outer() != self.unresolved_mark {
      // Don't process importCond more than once
    }

    let (Some((cond, _cond_span)), Some((if_true, if_true_span)), Some((if_false, if_false_span))) = (
      match_str(&call.args[0].expr),
      match_str(&call.args[1].expr),
      match_str(&call.args[2].expr),
    ) else {
      return;
    };

    let build_import = |atom: Atom, span: Span| -> Expr {
      CallExpr {
        callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
          "require".into(),
          // Required so that we resolve the new dependency
          DUMMY_SP.apply_mark(self.unresolved_mark),
        )))),
        args: vec![ExprOrSpread {
          expr: Box::new(Expr::Lit(Lit::Str(Str {
            value: atom,
            // This span is important to avoid getting marked as a helper
            span: span,
            raw: None,
          }))),
          spread: None,
        }],
        span: DUMMY_SP,
        type_args: None,
      }
      .into()
    };

    // importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
    // =>
    // (globalThis.__MOD_COND && globalThis.__MOD_COND['CONDITION'] ? require('IF_TRUE') : require('IF_FALSE')).default;
    let new_node = quote!(
      "(globalThis.__MOD_COND && globalThis.__MOD_COND[$cond] ? $if_true : $if_false).default" as Expr,
      cond: Expr = Expr::Lit(Lit::Str(cond.into())),
      if_true: Expr = build_import(if_true, if_true_span),
      if_false: Expr = build_import(if_false, if_false_span)
    );

    *node = new_node;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{remove_code_whitespace, run_test_visit, RunContext, RunVisitResult};

  fn make_conditional_imports<'a>(context: RunContext) -> ConditionalImportsFallback {
    ConditionalImportsFallback {
      unresolved_mark: context.unresolved_mark,
    }
  }

  #[test]
  fn test_import_cond() {
    let input_code = r#"
      const x = importCond('condition-1', 'a', 'b');
      const y = importCond('condition-2', 'c', 'd');
      const z = importCond('condition-2', 'c', 'd');
    "#;

    let RunVisitResult { output_code, .. } =
      run_test_visit(input_code, |context| make_conditional_imports(context));

    let expected_code = r#"
      const x = (globalThis.__MOD_COND && globalThis.__MOD_COND["condition-1"] ? require("a") : require("b")).default;
      const y = (globalThis.__MOD_COND && globalThis.__MOD_COND["condition-2"] ? require("c") : require("d")).default;
      const z = (globalThis.__MOD_COND && globalThis.__MOD_COND["condition-2"] ? require("c") : require("d")).default;
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }
}
