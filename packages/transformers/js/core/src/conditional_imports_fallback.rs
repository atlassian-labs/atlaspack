use swc_core::common::{Mark, Spanned};
use swc_core::ecma::ast::{
  BinExpr, BinaryOp, CallExpr, Callee, ComputedPropName, CondExpr, Expr, ExprOrSpread, Ident, Lit,
  MemberExpr, MemberProp, ParenExpr, Str,
};
use swc_core::ecma::visit::VisitMut;

use crate::utils::match_str;
use crate::Config;

pub struct ConditionalImportsFallback<'a> {
  pub config: &'a Config,
  pub unresolved_mark: Mark,
}

impl<'a> VisitMut for ConditionalImportsFallback<'a> {
  fn visit_mut_expr(&mut self, node: &mut Expr) {
    match node {
      Expr::Call(call) => match &call.callee {
        Callee::Expr(expr) => match &**expr {
          Expr::Ident(ident) if ident.sym.to_string().as_str() == "importCond" => {
            match (match_str(&call.args[1].expr), match_str(&call.args[2].expr)) {
              (Some((if_true, if_true_span)), Some((if_false, if_false_span))) => {
                if !self.config.conditional_bundling {
                  // Found importCond, if flag off replace an inline require import
                  // importCond('CONDITION', 'IF_TRUE', 'IF_FALSE');
                  // =>
                  // (globalThis.__MOD_COND && globalThis.__MOD_COND['CONDITION'] ? require('IF_TRUE') : require('IF_FALSE')).default;
                  *node = Expr::Member(MemberExpr {
                    obj: ParenExpr {
                      expr: CondExpr {
                        test: Box::new(
                          BinExpr {
                            op: BinaryOp::LogicalAnd,
                            left: MemberExpr {
                              obj: Box::new(Expr::Ident("globalThis".into())),
                              prop: MemberProp::Ident("__MOD_COND".into()),
                              span: call.span(),
                            }
                            .into(),
                            right: MemberExpr {
                              obj: Box::new(
                                MemberExpr {
                                  obj: Box::new(Expr::Ident("globalThis".into())),
                                  prop: MemberProp::Ident("__MOD_COND".into()),
                                  span: call.span(),
                                }
                                .into(),
                              ),
                              prop: MemberProp::Computed(ComputedPropName {
                                expr: call.args[0].expr.clone(),
                                span: call.args[0].expr.span(),
                              }),
                              span: call.span(),
                            }
                            .into(),
                            span: call.span(),
                          }
                          .into(),
                        ),
                        cons: Box::new(
                          CallExpr {
                            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                              "require".into(),
                              if_true_span.apply_mark(self.unresolved_mark),
                            )))),
                            args: vec![ExprOrSpread {
                              expr: Box::new(Expr::Lit(Lit::Str(Str {
                                value: if_true,
                                // This span is important to avoid getting marked as a helper
                                span: if_true_span,
                                raw: None,
                              }))),
                              spread: None,
                            }],
                            span: if_true_span,
                            type_args: None,
                          }
                          .into(),
                        ),
                        alt: Box::new(
                          CallExpr {
                            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
                              "require".into(),
                              if_false_span.apply_mark(self.unresolved_mark),
                            )))),
                            args: vec![ExprOrSpread {
                              expr: Box::new(Expr::Lit(Lit::Str(Str {
                                value: if_false,
                                // This span is important to avoid getting marked as a helper
                                span: if_false_span,
                                raw: None,
                              }))),
                              spread: None,
                            }],
                            span: if_false_span,
                            type_args: None,
                          }
                          .into(),
                        ),
                        span: call.span(),
                      }
                      .into(),
                      span: call.span(),
                    }
                    .into(),
                    prop: MemberProp::Ident("default".into()),
                    span: call.span(),
                  })
                }
              }
              _ => {}
            };
          }
          _ => {}
        },
        _ => {}
      },
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_utils::{remove_code_whitespace, run_test_visit, RunContext, RunVisitResult};

  fn make_conditional_imports<'a>(
    context: RunContext,
    config: &'a Config,
  ) -> ConditionalImportsFallback<'a> {
    ConditionalImportsFallback {
      config,
      unresolved_mark: context.unresolved_mark,
    }
  }

  fn make_config() -> Config {
    let mut config = Config::default();
    config.is_browser = true;
    config
  }

  #[test]
  fn test_import_cond_disabled() {
    let mut config = make_config();
    config.conditional_bundling = false;
    let input_code = r#"
      const x = importCond('condition-1', 'a', 'b');
      const y = importCond('condition-2', 'c', 'd');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_conditional_imports(context, &config)
    });

    let expected_code = r#"
      const x = (globalThis.__MOD_COND && globalThis.__MOD_COND['condition-1'] ? require("a") : require("b")).default;
      const y = (globalThis.__MOD_COND && globalThis.__MOD_COND['condition-2'] ? require("c") : require("d")).default;
    "#;

    assert_eq!(
      remove_code_whitespace(output_code.as_str()),
      remove_code_whitespace(expected_code)
    );
  }
}
